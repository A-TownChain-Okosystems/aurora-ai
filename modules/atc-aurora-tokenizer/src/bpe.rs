// Byte-Level-BPE (M-04). Vokabular 0..=255 sind die Byte-Werte selbst, jede
// weitere ID entsteht durch eine Paarung. Damit ist die Zerlegung verlustfrei
// und braucht keine Normalisierung (ATC-AI-LLM-001 §8 P1: "keine in v1").
//
// Determinismus ist keine Eigenschaft, die sich ergibt, sondern eine, die
// festgeschrieben wird: bei gleicher Paarhaeufigkeit gewinnt die lexikografisch
// kleinste Paarung, und die Iteration laeuft ueber `BTreeMap`, nicht ueber eine
// Hash-Map mit zufaelliger Reihenfolge.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::error::Error;
use crate::sha256;

/// Die 256 Byte-Token, die jedes Vokabular enthaelt.
pub const BASE_VOCAB_SIZE: usize = 256;
/// Harte Obergrenze. 4096 ist der Zielwert aus K4, nicht das Maximum des Formers.
const MAX_VOCAB_SIZE: usize = 65536;
/// Kopfzeile der kanonischen Form.
pub const CANONICAL_MAGIC: &str = "ATC-BPE 1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bpe {
    /// `tokens[id]` sind die Bytes, die dieses Token dekodiert.
    tokens: Vec<Vec<u8>>,
    /// `(links, rechts, neu)` in Trainingsreihenfolge.
    merges: Vec<(u32, u32, u32)>,
    /// Paarung -> ID. Die ID waechst mit der Trainingsreihenfolge, ist also
    /// gleichzeitig der Rang beim Kodieren.
    ranks: BTreeMap<(u32, u32), u32>,
}

impl Bpe {
    /// Vokabular aus den 256 Byte-Token, ohne Paarungen.
    pub fn base() -> Bpe {
        let mut tokens: Vec<Vec<u8>> = Vec::with_capacity(BASE_VOCAB_SIZE);
        for byte in 0..BASE_VOCAB_SIZE {
            tokens.push(alloc::vec![byte as u8]);
        }
        Bpe {
            tokens,
            merges: Vec::new(),
            ranks: BTreeMap::new(),
        }
    }

    pub fn vocab_size(&self) -> usize {
        self.tokens.len()
    }

    pub fn merges(&self) -> &[(u32, u32, u32)] {
        &self.merges
    }

    pub fn token_bytes(&self, id: u32) -> Option<&[u8]> {
        self.tokens.get(id as usize).map(|v| v.as_slice())
    }

    /// Trainiert ein Vokabular auf `corpus` bis `target_vocab` Token.
    ///
    /// Bricht typisiert ab, wenn das Ziel nicht erreichbar ist, weil keine
    /// Paarung mehr zweimal vorkommt: ein aufgefuelltes Vokabular mit
    /// Einmal-Paarungen waere eine Behauptung ohne Substanz.
    pub fn train(corpus: &[&[u8]], target_vocab: usize) -> Result<Bpe, Error> {
        if target_vocab < BASE_VOCAB_SIZE {
            return Err(Error::TargetVocabTooSmall(target_vocab));
        }
        if target_vocab > MAX_VOCAB_SIZE {
            return Err(Error::TargetVocabTooLarge(target_vocab));
        }

        let mut bpe = Bpe::base();
        let mut sequences: Vec<Vec<u32>> = corpus
            .iter()
            .map(|chunk| chunk.iter().map(|b| u32::from(*b)).collect())
            .collect();

        while bpe.tokens.len() < target_vocab {
            let mut counts: BTreeMap<(u32, u32), u32> = BTreeMap::new();
            for seq in &sequences {
                for pair in seq.windows(2) {
                    *counts.entry((pair[0], pair[1])).or_insert(0) += 1;
                }
            }

            // Hoechste Haeufigkeit; bei Gleichstand gewinnt die lexikografisch
            // kleinste Paarung, weil aufsteigend iteriert und nur bei echt
            // groesserem Zaehler ersetzt wird.
            let mut best: Option<((u32, u32), u32)> = None;
            for (pair, count) in &counts {
                if *count < 2 {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some((_, current)) => *count > current,
                };
                if better {
                    best = Some((*pair, *count));
                }
            }

            let Some(((left, right), _)) = best else {
                return Err(Error::TargetUnreachable {
                    reached: bpe.tokens.len(),
                    target: target_vocab,
                });
            };

            let new_id = bpe.tokens.len() as u32;
            let mut bytes = bpe.tokens[left as usize].clone();
            bytes.extend_from_slice(&bpe.tokens[right as usize]);
            bpe.tokens.push(bytes);
            bpe.merges.push((left, right, new_id));
            bpe.ranks.insert((left, right), new_id);

            for seq in sequences.iter_mut() {
                *seq = merge_pass(seq, left, right, new_id);
            }
        }

        Ok(bpe)
    }

    /// Zerlegt `input` in Token. Verlustfrei, weil jedes Token Bytes dekodiert
    /// und die Zerlegung eine Partition der Eingabe ist.
    pub fn encode(&self, input: &[u8]) -> Vec<u32> {
        let mut seq: Vec<u32> = input.iter().map(|b| u32::from(*b)).collect();
        if self.ranks.is_empty() {
            return seq;
        }
        loop {
            // Paarung mit dem kleinsten Rang suchen.
            let mut best: Option<u32> = None;
            for i in 0..seq.len().saturating_sub(1) {
                if let Some(new_id) = self.ranks.get(&(seq[i], seq[i + 1])) {
                    let better = match best {
                        None => true,
                        Some(current) => *new_id < current,
                    };
                    if better {
                        best = Some(*new_id);
                    }
                }
            }
            let Some(target) = best else { break };

            // Genau diese Paarung ueberall ersetzen, nicht ueberlappend.
            let mut next: Vec<u32> = Vec::with_capacity(seq.len());
            let mut i = 0;
            while i < seq.len() {
                if i + 1 < seq.len() && self.ranks.get(&(seq[i], seq[i + 1])) == Some(&target) {
                    next.push(target);
                    i += 2;
                } else {
                    next.push(seq[i]);
                    i += 1;
                }
            }
            seq = next;
        }
        seq
    }

    /// Bytes zu Token. Lehnt unbekannte IDs typisiert ab, statt zu raten.
    pub fn decode(&self, ids: &[u32]) -> Result<Vec<u8>, Error> {
        let mut out: Vec<u8> = Vec::new();
        for id in ids {
            let token = self.token_bytes(*id).ok_or(Error::UnknownToken(*id))?;
            out.extend_from_slice(token);
        }
        Ok(out)
    }

    /// Kanonische Form. Zeilenweise, ASCII, ohne Zeitstempel und ohne Pfad:
    /// derselbe Zustand ergibt immer dieselben Bytes und damit denselben Digest.
    pub fn canonical(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{CANONICAL_MAGIC}");
        let _ = writeln!(s, "vocab {}", self.tokens.len());
        for (id, bytes) in self.tokens.iter().enumerate() {
            let _ = write!(s, "{id} ");
            for byte in bytes {
                let _ = write!(s, "{byte:02x}");
            }
            s.push('\n');
        }
        let _ = writeln!(s, "merges {}", self.merges.len());
        for (left, right, new_id) in &self.merges {
            let _ = writeln!(s, "{left} {right} {new_id}");
        }
        s
    }

    /// SHA-256 ueber die kanonische Form, als Hex.
    pub fn digest_hex(&self) -> String {
        sha256::sha256_hex(self.canonical().as_bytes())
    }

    /// Laedt ein Vokabular aus der kanonischen Form und prueft dabei den Digest
    /// **vor** dem Parsen (fail-closed, B-17). Eine manipulierte Merge-Tabelle
    /// passiert diese Pruefung nicht.
    pub fn from_canonical(bytes: &[u8], expected_digest_hex: &str) -> Result<Bpe, Error> {
        let actual = sha256::sha256_hex(bytes);
        if !actual.eq_ignore_ascii_case(expected_digest_hex) {
            return Err(Error::DigestMismatch);
        }
        let text = core::str::from_utf8(bytes).map_err(|_| Error::Malformed("kein UTF-8"))?;
        Self::parse(text)
    }

    fn parse(text: &str) -> Result<Bpe, Error> {
        let mut lines = text.lines();
        if lines.next() != Some(CANONICAL_MAGIC) {
            return Err(Error::Malformed("Kopfzeile"));
        }

        let vocab_header = lines.next().ok_or(Error::Malformed("vocab fehlt"))?;
        let vocab_count = parse_count(vocab_header, "vocab ")?;
        if vocab_count < BASE_VOCAB_SIZE {
            return Err(Error::Malformed("Vokabular unter 256"));
        }

        let mut tokens: Vec<Vec<u8>> = Vec::with_capacity(vocab_count);
        for expected_id in 0..vocab_count {
            let line = lines.next().ok_or(Error::Malformed("Token-Zeile fehlt"))?;
            let (id_part, hex_part) = line
                .split_once(' ')
                .ok_or(Error::Malformed("Token-Zeile ohne Trenner"))?;
            let id: usize = id_part.parse().map_err(|_| Error::Malformed("Token-ID"))?;
            if id != expected_id {
                return Err(Error::Malformed("Token-IDs nicht fortlaufend"));
            }
            tokens.push(parse_hex(hex_part)?);
        }

        let merges_header = lines.next().ok_or(Error::Malformed("merges fehlt"))?;
        let merge_count = parse_count(merges_header, "merges ")?;
        if merge_count + BASE_VOCAB_SIZE != vocab_count {
            return Err(Error::Malformed("merges passen nicht zum Vokabular"));
        }

        let mut merges: Vec<(u32, u32, u32)> = Vec::with_capacity(merge_count);
        let mut ranks: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for step in 0..merge_count {
            let line = lines.next().ok_or(Error::Malformed("Merge-Zeile fehlt"))?;
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() != 3 {
                return Err(Error::Malformed("Merge-Zeile braucht drei Felder"));
            }
            let left: u32 = parts[0]
                .parse()
                .map_err(|_| Error::Malformed("linkes Feld"))?;
            let right: u32 = parts[1]
                .parse()
                .map_err(|_| Error::Malformed("rechtes Feld"))?;
            let new_id: u32 = parts[2].parse().map_err(|_| Error::Malformed("neue ID"))?;
            let expected = (BASE_VOCAB_SIZE + step) as u32;
            if new_id != expected {
                return Err(Error::Malformed("Merge-IDs nicht fortlaufend"));
            }
            if left as usize >= tokens.len() || right as usize >= tokens.len() {
                return Err(Error::Malformed("Merge verweist auf unbekanntes Token"));
            }
            let mut joined = tokens[left as usize].clone();
            joined.extend_from_slice(&tokens[right as usize]);
            if joined != tokens[new_id as usize] {
                return Err(Error::Malformed("Merge-Bytes stimmen nicht"));
            }
            merges.push((left, right, new_id));
            ranks.insert((left, right), new_id);
        }

        if lines.next().is_some() {
            return Err(Error::Malformed("ueberzaehlige Zeilen"));
        }

        Ok(Bpe {
            tokens,
            merges,
            ranks,
        })
    }
}

fn merge_pass(seq: &[u32], left: u32, right: u32, new_id: u32) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::with_capacity(seq.len());
    let mut i = 0;
    while i < seq.len() {
        if i + 1 < seq.len() && seq[i] == left && seq[i + 1] == right {
            out.push(new_id);
            i += 2;
        } else {
            out.push(seq[i]);
            i += 1;
        }
    }
    out
}

fn parse_count(header: &str, prefix: &str) -> Result<usize, Error> {
    let rest = header
        .strip_prefix(prefix)
        .ok_or(Error::Malformed("Kopfzeile vocab/merges"))?;
    rest.parse().map_err(|_| Error::Malformed("Anzahl"))
}

fn parse_hex(text: &str) -> Result<Vec<u8>, Error> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return Err(Error::Malformed("Hex-Feld"));
    }
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_value(bytes[i])?;
        let lo = hex_value(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_value(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::Malformed("Hex-Ziffer")),
    }
}

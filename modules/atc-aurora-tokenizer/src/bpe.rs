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

/// Fehlerklassen nach `ATC-AI-LLM-004` §10. Die Kennung steht am Anfang des
/// Textes, damit ein Test die Klasse pruefen kann, ohne den Wortlaut zu binden.
pub const E_KOPFZEILE: &str = "E-01 Kopfzeile oder Fassung unbekannt";
pub const E_VOCAB_KOPF: &str = "E-02 vocab-Kopfzeile fehlerhaft";
pub const E_VOCAB_BEREICH: &str = "E-03 Vokabulargroesse ausserhalb 256..=65536";
pub const E_TOKEN_ZEILE: &str = "E-04 Token-Zeile fehlerhaft";
pub const E_TOKEN_ID: &str = "E-05 Token-IDs nicht lueckenlos aufsteigend";
pub const E_BASIS: &str = "E-06 Basisidentitaet verletzt";
pub const E_HEX: &str = "E-07 Hex-Feld unzulaessig";
pub const E_MERGES_KOPF: &str = "E-08 merges-Kopfzeile fehlerhaft";
pub const E_MERGES_ANZAHL: &str = "E-09 Merge-Anzahl passt nicht zum Vokabular";
pub const E_MERGE_ZEILE: &str = "E-10 Merge-Zeile fehlerhaft";
pub const E_MERGE_ID: &str = "E-11 Merge-IDs nicht fortlaufend ab 256";
pub const E_MERGE_REF: &str = "E-12 Merge verweist auf unbekanntes Token";
pub const E_MERGE_BYTES: &str = "E-13 Merge-Bytes stimmen nicht";
pub const E_PAAR_DOPPELT: &str = "E-14 Paar doppelt vorhanden";
pub const E_UEBERZAEHLIG: &str = "E-15 ueberzaehlige Zeilen";
pub const E_NICHT_KANONISCH: &str = "E-16 nicht kanonisch";
pub const E_DIGEST_FORMAT: &str = "E-17 Digestformat unzulaessig";
pub const E_ZEICHENSATZ: &str = "E-18 Zeichensatz oder Zeilenende unzulaessig";

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

    /// Laedt ein Vokabular aus der kanonischen Form nach `ATC-AI-LLM-004` (K-T1).
    ///
    /// Pruefreihenfolge nach Klausel C-24, fail-closed: erst das Digestformat,
    /// dann der Digest gegen die Bytes, dann Zeichensatz, Struktur und zum
    /// Schluss die Kanonizitaet durch Re-Serialisierung. Eine manipulierte Datei
    /// wird bei Schritt 2 abgewiesen, ohne dass ihre Struktur interpretiert wurde.
    pub fn from_canonical(bytes: &[u8], expected_digest_hex: &str) -> Result<Bpe, Error> {
        // C-22 / C-24-1: Digestformat, 64 Zeichen kleingeschriebenes Hex.
        if !ist_digestformat(expected_digest_hex) {
            return Err(Error::Malformed(E_DIGEST_FORMAT));
        }
        // C-21 / C-23 / C-24-2: Digest ueber genau diese Bytes, exakter Vergleich.
        let actual = sha256::sha256_hex(bytes);
        if actual != expected_digest_hex {
            return Err(Error::DigestMismatch);
        }
        // C-03 / C-24-3: ASCII, nur LF als Zeilenende.
        if !bytes
            .iter()
            .all(|b| *b == b'\n' || (0x20..=0x7E).contains(b))
        {
            return Err(Error::Malformed(E_ZEICHENSATZ));
        }
        let text = core::str::from_utf8(bytes).map_err(|_| Error::Malformed(E_ZEICHENSATZ))?;
        // C-24-4 bis C-24-9
        let bpe = Self::parse(text)?;
        // C-24-10 / C-05: Sicherheitsnetz, Import akzeptiert nie mehr als Export erzeugt.
        if bpe.canonical().as_bytes() != bytes {
            return Err(Error::Malformed(E_NICHT_KANONISCH));
        }
        Ok(bpe)
    }

    fn parse(text: &str) -> Result<Bpe, Error> {
        let mut lines = text.lines();
        if lines.next() != Some(CANONICAL_MAGIC) {
            return Err(Error::Malformed(E_KOPFZEILE));
        }

        // C-02 / C-20 / C-08
        let vocab_header = lines.next().ok_or(Error::Malformed(E_VOCAB_KOPF))?;
        let vocab_field = vocab_header
            .strip_prefix("vocab ")
            .ok_or(Error::Malformed(E_VOCAB_KOPF))?;
        let vocab_count = parse_decimal(vocab_field, E_VOCAB_KOPF)? as usize;
        if !(BASE_VOCAB_SIZE..=MAX_VOCAB_SIZE).contains(&vocab_count) {
            return Err(Error::Malformed(E_VOCAB_BEREICH));
        }

        // C-04 bis C-11
        let mut tokens: Vec<Vec<u8>> = Vec::with_capacity(vocab_count);
        for expected_id in 0..vocab_count {
            let line = lines.next().ok_or(Error::Malformed(E_TOKEN_ZEILE))?;
            let (id_part, hex_part) = line
                .split_once(' ')
                .ok_or(Error::Malformed(E_TOKEN_ZEILE))?;
            let id = parse_decimal(id_part, E_TOKEN_ZEILE)? as usize;
            if id != expected_id {
                return Err(Error::Malformed(E_TOKEN_ID));
            }
            let bytes_of_token = parse_hex(hex_part)?;
            if expected_id < BASE_VOCAB_SIZE && bytes_of_token != alloc::vec![expected_id as u8] {
                return Err(Error::Malformed(E_BASIS));
            }
            tokens.push(bytes_of_token);
        }

        // C-13 / C-14
        let merges_header = lines.next().ok_or(Error::Malformed(E_MERGES_KOPF))?;
        let merge_field = merges_header
            .strip_prefix("merges ")
            .ok_or(Error::Malformed(E_MERGES_KOPF))?;
        let merge_count = parse_decimal(merge_field, E_MERGES_KOPF)? as usize;
        if merge_count + BASE_VOCAB_SIZE != vocab_count {
            return Err(Error::Malformed(E_MERGES_ANZAHL));
        }

        // C-15 bis C-17
        let mut merges: Vec<(u32, u32, u32)> = Vec::with_capacity(merge_count);
        let mut ranks: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for step in 0..merge_count {
            let line = lines.next().ok_or(Error::Malformed(E_MERGE_ZEILE))?;
            let felder: Vec<&str> = line.split(' ').collect();
            if felder.len() != 3 || felder.iter().any(|f| f.is_empty()) {
                return Err(Error::Malformed(E_MERGE_ZEILE));
            }
            let left = parse_decimal(felder[0], E_MERGE_ZEILE)?;
            let right = parse_decimal(felder[1], E_MERGE_ZEILE)?;
            let new_id = parse_decimal(felder[2], E_MERGE_ZEILE)?;

            let expected_new = (BASE_VOCAB_SIZE + step) as u32;
            if new_id != expected_new {
                return Err(Error::Malformed(E_MERGE_ID));
            }
            let grenze = expected_new;
            if left >= grenze || right >= grenze {
                return Err(Error::Malformed(E_MERGE_REF));
            }
            if ranks.contains_key(&(left, right)) {
                return Err(Error::Malformed(E_PAAR_DOPPELT));
            }
            let mut joined = tokens[left as usize].clone();
            joined.extend_from_slice(&tokens[right as usize]);
            if joined != tokens[new_id as usize] {
                return Err(Error::Malformed(E_MERGE_BYTES));
            }
            merges.push((left, right, new_id));
            ranks.insert((left, right), new_id);
        }

        // C-07
        if lines.next().is_some() {
            return Err(Error::Malformed(E_UEBERZAEHLIG));
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

/// C-20: Dezimalfeld ist `0` oder `[1-9][0-9]*`. Kein Vorzeichen, keine fuehrende
/// Null, kein Leerzeichen, kein leeres Feld.
fn parse_decimal(field: &str, code: &'static str) -> Result<u32, Error> {
    let bytes = field.as_bytes();
    if bytes.is_empty() || !bytes.iter().all(|b| b.is_ascii_digit()) {
        return Err(Error::Malformed(code));
    }
    if bytes.len() > 1 && bytes[0] == b'0' {
        return Err(Error::Malformed(code));
    }
    if bytes.len() > 5 {
        return Err(Error::Malformed(code));
    }
    field.parse::<u32>().map_err(|_| Error::Malformed(code))
}

/// C-10: Hex kleingeschrieben, gerade Stellenzahl, mindestens zwei Stellen.
fn parse_hex(text: &str) -> Result<Vec<u8>, Error> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return Err(Error::Malformed(E_HEX));
    }
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

/// Nur `0-9a-f`. Grossbuchstaben sind kein Stilunterschied, sondern ein Fehler.
fn hex_value(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(Error::Malformed(E_HEX)),
    }
}

/// C-22: 64 Zeichen, ausschliesslich `0-9a-f`.
fn ist_digestformat(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// Pruefungen zu P1 (M-04). Die Nummern B-14 bis B-17 sind die des Blueprints
// `ATC-AI-LLM-001-B` §3.4, die Austrittskriterien die von `ATC-AI-LLM-001` §8 P1.

use alloc::string::String;
use alloc::vec::Vec;

use crate::bpe::{Bpe, BASE_VOCAB_SIZE};
use crate::error::Error;
use crate::sha256;

// ---------------------------------------------------------------- Hilfsmittel

/// SplitMix64: deterministische Zufallsfolge ohne Abhaengigkeit. Ein Test, der
/// bei jedem Lauf andere Bytes zieht, beweist nichts ueber Reproduzierbarkeit.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            out.extend_from_slice(&self.next_u64().to_le_bytes());
        }
        out.truncate(len);
        out
    }
}

fn sample_corpus() -> Vec<Vec<u8>> {
    let texte: [&[u8]; 14] = [
        b"aurora ist ein eigenes modell und kein fremdes framework",
        b"der tokenizer zerlegt bytes verlustfrei und deterministisch",
        b"deterministisch heisst: gleiche quelle, gleiches vokabular, gleicher digest",
        b"keine normalisierung in v1, keine stille ersetzung durch ein ersatzzeichen",
        "äöü ß 日本語 emoji 🙂 und ein vierbyte-zeichen 🦀".as_bytes(),
        b"die merge-tabelle ist digest-gebunden und wird vor dem parsen geprueft",
        b"ein byte-level-vokabular beginnt bei zweihundertsechsundfuenfzig token",
        b"paarhaeufigkeit absteigend, bei gleichstand die kleinere paarung",
        b"das vokabular waechst nur, solange eine paarung zweimal vorkommt",
        b"training ohne festen tie-break ist nicht reproduzierbar und damit wertlos",
        b"rundlauf ueber zehntausend zufallsfolgen, auch fuer ungueltiges utf-8",
        b"kein panik, kein stilles ersatzzeichen, kein raten bei unbekannten ids",
        b"der erste block traegt die byte-token, danach folgen die merge-zeilen",
        b"aurora llm 1b prototyp 1036077056 parameter d_ff 5632",
    ];
    texte.iter().map(|t| t.to_vec()).collect()
}

fn train_sample() -> Bpe {
    let corpus = sample_corpus();
    let refs: Vec<&[u8]> = corpus.iter().map(|c| c.as_slice()).collect();
    Bpe::train(&refs, 380).expect("Trainingskorpus ist zulaessig")
}

// ------------------------------------------------------------------- SHA-256

#[test]
fn sha256_trifft_die_bekannten_vektoren() {
    assert_eq!(
        sha256::sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256::sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

// --------------------------------------------------------- Grundzustand, B-16

#[test]
fn basisvokabular_zerlegt_in_einzelbytes() {
    let bpe = Bpe::base();
    assert_eq!(bpe.vocab_size(), BASE_VOCAB_SIZE);
    assert!(bpe.merges().is_empty());
    for byte in 0u16..=255 {
        let b = [byte as u8];
        let ids = bpe.encode(&b);
        assert_eq!(ids, alloc::vec![u32::from(byte as u8)]);
        assert_eq!(bpe.decode(&ids).unwrap(), b);
    }
}

#[test]
fn b_16_grenzfaelle_ohne_panik_und_ohne_ersatzzeichen() {
    let bpe = train_sample();
    let mut rng = Rng::new(0xA0_11_0F_1E);
    let ein_mb = rng.bytes(1024 * 1024);
    let faelle: [&[u8]; 7] = [
        b"",
        &[0x00],
        &[0xFF],
        "\u{1F642}".as_bytes(),                // Emoji, 4 Byte UTF-8
        &[0xF0, 0x9F, 0xA6, 0x80],             // 4-Byte-Sequenz als Rohbytes
        &[0xFF, 0xFE, 0xC0, 0x80, 0xED, 0xA0], // ungueltiges UTF-8
        &ein_mb,
    ];
    for fall in faelle {
        let ids = bpe.encode(fall);
        let back = bpe.decode(&ids).expect("eigene IDs");
        assert_eq!(back, fall);
        // Kein stilles Ersatzzeichen: U+FFFD duerfte nur stehen, wenn es in der
        // Eingabe stand.
        let hat_ersatz = fall.windows(3).any(|w| w == [0xEF, 0xBF, 0xBD]);
        let back_hat_ersatz = back.windows(3).any(|w| w == [0xEF, 0xBF, 0xBD]);
        assert_eq!(hat_ersatz, back_hat_ersatz);
    }
}

// ------------------------------------------------------------ Determinismus B-15

#[test]
fn b_15_zwei_laeufe_ergeben_identisches_vokabular_und_digest() {
    let a = train_sample();
    let b = train_sample();
    assert_eq!(a.vocab_size(), b.vocab_size());
    assert_eq!(a.merges(), b.merges());
    assert_eq!(a.canonical(), b.canonical());
    assert_eq!(a.digest_hex(), b.digest_hex());
}

#[test]
fn b_15_merkmal_stabil_ueber_wiederholte_laeufe() {
    // Dritter Lauf, andere Reihenfolge der gleichen Quelle im Speicher: die
    // Quelle ist die Menge der Texte, nicht ihre Adresse.
    let mut corpus = sample_corpus();
    corpus.reverse();
    let refs: Vec<&[u8]> = corpus.iter().map(|c| c.as_slice()).collect();
    let c = Bpe::train(&refs, 380).unwrap();
    let a = train_sample();
    assert_eq!(
        a.digest_hex(),
        c.digest_hex(),
        "Reihenfolge der Quelle darf nichts aendern"
    );

    let nochmal = train_sample();
    assert_eq!(a.digest_hex(), nochmal.digest_hex());
}

#[test]
fn tie_break_ist_die_lexikografisch_kleinste_paarung() {
    // "abab": (a,b) kommt zweimal vor, (b,a) einmal. Erste Paarung muss (97,98) sein.
    let bpe = Bpe::train(&[b"abab"], 257).expect("ein Merge ist erreichbar");
    assert_eq!(bpe.merges()[0], (97, 98, 256));
    assert_eq!(bpe.vocab_size(), 257);
    // Danach bleibt (256,256) mit einem Vorkommen: kein zweites Merge moeglich.
    assert!(matches!(
        Bpe::train(&[b"abab"], 258),
        Err(Error::TargetUnreachable {
            reached: 257,
            target: 258
        })
    ));
}

#[test]
fn training_bricht_typisiert_ab_wenn_das_ziel_nicht_erreichbar_ist() {
    match Bpe::train(&[b"a"], 300) {
        Err(Error::TargetUnreachable { reached, target }) => {
            assert_eq!(reached, 256);
            assert_eq!(target, 300);
        }
        other => panic!("erwartet TargetUnreachable, war {other:?}"),
    }
    assert_eq!(
        Bpe::train(&[b"abc"], 100),
        Err(Error::TargetVocabTooSmall(100))
    );
    assert_eq!(
        Bpe::train(&[b"abc"], 100_000),
        Err(Error::TargetVocabTooLarge(100_000))
    );
}

// -------------------------------------------------------------- Rundlauf B-14

#[test]
fn b_14_rundlauf_ueber_zehntausend_zufallsfolgen() {
    let bpe = train_sample();
    let mut rng = Rng::new(0x00B1_4C0D);
    for lauf in 0..10_000 {
        let len = (rng.next_u64() % 65) as usize;
        let folge = rng.bytes(len);
        let ids = bpe.encode(&folge);
        let back = bpe.decode(&ids).expect("eigene IDs");
        assert_eq!(back, folge, "Lauf {lauf}, {len} Bytes");
    }
}

#[test]
fn b_14_rundlauf_ueber_text_und_zufall_gemischt() {
    let bpe = train_sample();
    let mut rng = Rng::new(0x5EED_1234);
    for vorlage in sample_corpus() {
        let muell = rng.bytes(vorlage.len() % 17);
        let mut gemischt = vorlage.clone();
        gemischt.extend_from_slice(&muell);
        let ids = bpe.encode(&gemischt);
        assert_eq!(bpe.decode(&ids).unwrap(), gemischt);
    }
}

// ------------------------------------------------- 20 Testvektoren aus §8 P1

#[test]
fn zwanzig_testvektoren_rundlauf_und_stabilitaet() {
    let bpe = train_sample();
    let mut rng = Rng::new(0xC0_FF_EE_01);
    let ein_mb = rng.bytes(1024 * 1024);
    let vektoren: [(&str, &[u8]); 20] = [
        ("leer", b""),
        ("ein_byte_null", &[0x00]),
        ("ein_byte_ff", &[0xFF]),
        ("ascii_wort", b"aurora"),
        ("ascii_satz", b"der tokenizer ist digest-gebunden"),
        ("grossbuchstaben", b"AURORA LLM 1B"),
        ("ziffern", b"1036077056"),
        ("umlaut", "äöü".as_bytes()),
        ("scharfes_s", "ß".as_bytes()),
        ("latein_mit_akzent", "é".as_bytes()),
        ("zweibyte_utf8", "\u{00E4}".as_bytes()),
        ("dreibe_yte_utf8", "\u{20AC}".as_bytes()),
        ("emoji", "\u{1F642}".as_bytes()),
        ("vierbyte_zeichen", "\u{1F980}".as_bytes()),
        ("gemischt_script", "日本語 und latin".as_bytes()),
        (
            "ungueltiges_utf8",
            &[0xC3, 0x28, 0xA0, 0xA1, 0xE2, 0x28, 0xA1],
        ),
        ("nur_nullbytes", &[0x00; 32]),
        ("wiederholung", b"abababababababab"),
        ("crlf", b"zeile1\r\nzeile2\r\n"),
        ("ein_megabyte_zufall", &ein_mb),
    ];
    assert_eq!(vektoren.len(), 20);
    for (name, eingabe) in vektoren {
        let ids = bpe.encode(eingabe);
        let back = bpe.decode(&ids).expect("eigene IDs");
        assert_eq!(back, eingabe, "Vektor {name}");
        // Stabilitaet: zweite Zerlegung derselben Eingabe ist identisch.
        assert_eq!(bpe.encode(eingabe), ids, "Vektor {name} nicht stabil");
        if eingabe.is_empty() {
            assert!(ids.is_empty(), "leere Eingabe ergibt keine Token");
        }
    }
}

// ------------------------------------------------------------ Artefakt, B-17

#[test]
fn kanonische_form_laedt_sich_selbst() {
    let bpe = train_sample();
    let canon = bpe.canonical();
    let digest = bpe.digest_hex();
    let geladen = Bpe::from_canonical(canon.as_bytes(), &digest).expect("eigener Digest");
    assert_eq!(geladen.canonical(), canon);
    assert_eq!(geladen.digest_hex(), digest);
    assert_eq!(geladen.merges(), bpe.merges());
    // Und die geladene Tabelle kodiert identisch.
    for vorlage in sample_corpus() {
        assert_eq!(geladen.encode(&vorlage), bpe.encode(&vorlage));
    }
}

#[test]
fn b_17_manipulierte_merge_tabelle_wird_abgelehnt() {
    let bpe = train_sample();
    let canon = bpe.canonical();
    let digest = bpe.digest_hex();

    // Eine Merge-Zeile veraendern, Digest bleibt der alte.
    let mut zeilen: Vec<String> = canon.lines().map(String::from).collect();
    let merge_index = zeilen
        .iter()
        .position(|l| l.starts_with("merges "))
        .expect("Kopfzeile merges");
    let erste = zeilen[merge_index + 1].clone();
    let teile: Vec<&str> = erste.split(' ').collect();
    let manipuliert = alloc::format!("{} {} {}", teile[1], teile[0], teile[2]);
    zeilen[merge_index + 1] = manipuliert;
    let mut manipuliert_bytes = String::new();
    for zeile in &zeilen {
        manipuliert_bytes.push_str(zeile);
        manipuliert_bytes.push('\n');
    }

    assert_eq!(
        Bpe::from_canonical(manipuliert_bytes.as_bytes(), &digest),
        Err(Error::DigestMismatch),
        "manipulierte Tabelle darf den alten Digest nicht passieren"
    );

    // Auch mit passendem Digest bleibt die Tabelle strukturell falsch:
    // die Merge-Bytes stimmen dann nicht mehr.
    let manipulierter_digest = sha256::sha256_hex(manipuliert_bytes.as_bytes());
    assert_code(
        Bpe::from_canonical(manipuliert_bytes.as_bytes(), &manipulierter_digest),
        "E-13",
    );
}

#[test]
fn b_17_kaputte_kopfzeile_wird_abgelehnt() {
    let kaputt = b"ATC-BPE 2\nvocab 256\nmerges 0\n".to_vec();
    let digest = sha256::sha256_hex(&kaputt);
    assert_code(Bpe::from_canonical(&kaputt, &digest), "E-01");
}

#[test]
fn unbekanntes_token_ist_ein_fehler_und_kein_raten() {
    let bpe = Bpe::base();
    assert_eq!(bpe.decode(&[256]), Err(Error::UnknownToken(256)));
    assert_eq!(bpe.decode(&[0, 255]), Ok(alloc::vec![0u8, 255]));
}

#[test]
fn vokabular_und_digest_haengen_zusammen() {
    // Unterschiedliche Ziele ergeben unterschiedliche Digests: der Digest ist
    // die Identitaet des Artefakts, nicht eine Verzierung.
    let corpus = sample_corpus();
    let refs: Vec<&[u8]> = corpus.iter().map(|c| c.as_slice()).collect();
    let klein = Bpe::train(&refs, 300).unwrap();
    let gross = Bpe::train(&refs, 380).unwrap();
    assert_ne!(klein.digest_hex(), gross.digest_hex());
    assert_eq!(klein.vocab_size(), 300);
    assert_eq!(gross.vocab_size(), 380);
}

// --------------------------------------- Artefakte für die Vertragsprüfungen

fn ist_code(err: &Error, code: &str) -> bool {
    matches!(err, Error::Malformed(text) if text.starts_with(code))
}

fn assert_code(res: Result<Bpe, Error>, code: &str) {
    match res {
        Err(err) if ist_code(&err, code) => {}
        other => panic!("erwartet {code}, war {other:?}"),
    }
}

fn assert_abgewiesen(res: Result<Bpe, Error>) {
    assert!(res.is_err(), "Artefakt haette abgewiesen werden muessen");
}

fn als_bytes(zeilen: &[String]) -> Vec<u8> {
    let mut text = String::new();
    for zeile in zeilen {
        text.push_str(zeile);
        text.push('\n');
    }
    text.into_bytes()
}

/// Bytes plus der dazugehoerige Digest: die Strukturpruefung wird getestet, nicht
/// die Digestpruefung.
fn lade(zeilen: &[String]) -> Result<Bpe, Error> {
    let bytes = als_bytes(zeilen);
    let digest = sha256::sha256_hex(&bytes);
    Bpe::from_canonical(&bytes, &digest)
}

fn zeilen(canon: &str) -> Vec<String> {
    canon.lines().map(String::from).collect()
}

/// Basisartefakt (256 Byte-Token, keine Merges) plus frei gewaehlte Zusatzzeilen.
fn handartefakt(zusatz_tokens: &[&str], merges: &[&str]) -> Vec<String> {
    let mut zeilen: Vec<String> = Vec::new();
    zeilen.push(String::from("ATC-BPE 1"));
    zeilen.push(alloc::format!(
        "vocab {}",
        BASE_VOCAB_SIZE + zusatz_tokens.len()
    ));
    for id in 0..BASE_VOCAB_SIZE {
        zeilen.push(alloc::format!("{id} {id:02x}"));
    }
    for (i, token) in zusatz_tokens.iter().enumerate() {
        zeilen.push(alloc::format!("{} {token}", BASE_VOCAB_SIZE + i));
    }
    zeilen.push(alloc::format!("merges {}", merges.len()));
    for merge in merges {
        zeilen.push(String::from(*merge));
    }
    zeilen
}

fn basis() -> Vec<String> {
    zeilen(&Bpe::base().canonical())
}

// ------------------------------------- Klauseln C-03 bis C-24 (ATC-AI-LLM-004)

#[test]
fn c_03_zeichensatz_und_cr_werden_abgewiesen() {
    let mut bytes = als_bytes(&basis());
    // CRLF statt LF
    bytes = bytes
        .iter()
        .flat_map(|b| {
            if *b == b'\n' {
                alloc::vec![b'\r', b'\n']
            } else {
                alloc::vec![*b]
            }
        })
        .collect();
    let digest = sha256::sha256_hex(&bytes);
    assert_code(Bpe::from_canonical(&bytes, &digest), "E-18");

    // Nicht-ASCII
    let mut bytes = als_bytes(&basis());
    let stelle = bytes.len() / 2;
    bytes[stelle] = 0xC3;
    let digest = sha256::sha256_hex(&bytes);
    assert_code(Bpe::from_canonical(&bytes, &digest), "E-18");
}

#[test]
fn c_05_fehlendes_abschliessendes_lf_ist_nicht_kanonisch() {
    let mut bytes = als_bytes(&basis());
    assert_eq!(bytes.pop(), Some(b'\n'));
    let digest = sha256::sha256_hex(&bytes);
    assert_code(Bpe::from_canonical(&bytes, &digest), "E-16");
}

#[test]
fn c_06_doppeltes_leerzeichen_wird_abgewiesen() {
    let mut zeilen = basis();
    zeilen[2] = String::from("0  00");
    assert_abgewiesen(lade(&zeilen));
}

#[test]
fn c_07_ueberzaehlige_zeile_wird_abgewiesen() {
    let mut zeilen = basis();
    zeilen.push(String::from("kommentar"));
    assert_code(lade(&zeilen), "E-15");
}

#[test]
fn c_09_token_ids_muessen_fortlaufend_sein() {
    let mut zeilen = basis();
    zeilen.swap(2, 3);
    assert_code(lade(&zeilen), "E-05");
}

#[test]
fn c_10_grossbuchstaben_im_hexfeld_werden_abgewiesen() {
    let mut zeilen = basis();
    zeilen[2] = String::from("0 0A");
    assert_code(lade(&zeilen), "E-07");
}

#[test]
fn c_11_basisidentitaet_wird_geprueft() {
    let mut zeilen = basis();
    zeilen[2] = String::from("0 01");
    assert_code(lade(&zeilen), "E-06");
}

#[test]
fn c_13_merge_zeile_mit_zwei_feldern_wird_abgewiesen() {
    let mut zeilen = handartefakt(&["6162"], &["97 98 256"]);
    let letzte = zeilen.len() - 1;
    zeilen[letzte] = String::from("97 98");
    assert_code(lade(&zeilen), "E-10");
}

#[test]
fn c_14_merge_anzahl_passt_nicht_zum_vokabular() {
    let mut zeilen = handartefakt(&["6162"], &["97 98 256"]);
    let kopf = zeilen.len() - 2;
    zeilen[kopf] = String::from("merges 0");
    assert_code(lade(&zeilen), "E-09");
}

#[test]
fn c_15_merge_ids_muessen_fortlaufend_sein() {
    let zeilen = handartefakt(&["6162"], &["97 98 257"]);
    assert_code(lade(&zeilen), "E-11");
}

#[test]
fn c_16_vorwaertsverweis_wird_abgewiesen() {
    let zeilen = handartefakt(&["6162"], &["97 256 256"]);
    assert_code(lade(&zeilen), "E-12");
}

#[test]
fn c_17_doppeltes_paar_wird_abgewiesen() {
    let zeilen = handartefakt(&["6162", "6162"], &["97 98 256", "97 98 257"]);
    assert_code(lade(&zeilen), "E-14");
}

#[test]
fn c_18_reihenfolge_aendert_den_digest() {
    // Dieselben Paare, dieselben Bytes, andere Reihenfolge.
    let a = Bpe::from_canonical(
        &als_bytes(&handartefakt(
            &["6162", "616263"],
            &["97 98 256", "256 99 257"],
        )),
        &sha256::sha256_hex(&als_bytes(&handartefakt(
            &["6162", "616263"],
            &["97 98 256", "256 99 257"],
        ))),
    )
    .expect("A ist gueltig");
    let b = Bpe::from_canonical(
        &als_bytes(&handartefakt(
            &["6263", "616263"],
            &["98 99 256", "97 256 257"],
        )),
        &sha256::sha256_hex(&als_bytes(&handartefakt(
            &["6263", "616263"],
            &["98 99 256", "97 256 257"],
        ))),
    )
    .expect("B ist gueltig");
    assert_ne!(a.digest_hex(), b.digest_hex());
}

#[test]
fn c_20_fuehrende_null_wird_abgewiesen() {
    let mut zeilen = basis();
    zeilen[1] = String::from("vocab 0256");
    assert_code(lade(&zeilen), "E-02");
}

#[test]
fn c_22_digestformat_wird_geprueft() {
    let bytes = als_bytes(&basis());
    let digest = sha256::sha256_hex(&bytes);
    let gross = digest.to_uppercase();
    assert_code(Bpe::from_canonical(&bytes, &gross), "E-17");
    assert_code(Bpe::from_canonical(&bytes, &digest[..63]), "E-17");
    assert_code(
        Bpe::from_canonical(&bytes, &alloc::format!("{digest} ")),
        "E-17",
    );
}

#[test]
fn c_23_digest_vergleich_ist_exakt() {
    let bytes = als_bytes(&basis());
    let digest = sha256::sha256_hex(&bytes);
    let mut verfaelscht: Vec<char> = digest.chars().collect();
    verfaelscht[0] = if verfaelscht[0] == 'a' { 'b' } else { 'a' };
    let verfaelscht: String = verfaelscht.into_iter().collect();
    assert_eq!(
        Bpe::from_canonical(&bytes, &verfaelscht),
        Err(Error::DigestMismatch)
    );
}

#[test]
fn c_24_digest_vor_struktur() {
    // Strukturell kaputte Datei, aber mit dem Digest der kaputten Bytes: die
    // Struktur wird geprueft. Mit dem Digest der heilen Bytes: nur der Digest.
    let heil = als_bytes(&basis());
    let heiler_digest = sha256::sha256_hex(&heil);
    let mut kaputt = heil.clone();
    kaputt[0] = b'X';
    assert_eq!(
        Bpe::from_canonical(&kaputt, &heiler_digest),
        Err(Error::DigestMismatch),
        "Digest wird vor der Struktur geprueft"
    );
}

#[test]
fn vertragliche_reihenfolge_der_fehlerklassen() {
    // Digestformat vor Digest: ein Artefakt mit kaputtem Digestformat und
    // falschem Digest meldet das Format, nicht den Mismatch.
    let bytes = als_bytes(&basis());
    assert_code(Bpe::from_canonical(&bytes, "nicht-hex"), "E-17");
}

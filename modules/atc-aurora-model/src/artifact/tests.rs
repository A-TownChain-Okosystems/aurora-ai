//! Adversariale Pruefungen des Artefaktformats (AT-01 bis AT-35) und der
//! Digest-Implementierung.
//!
//! Jeder Fall ist eine gezielte Mutation eines gueltigen Artefakts. Erwartet
//! wird ein typisierter Fehler: keine Panik, keine Allokation anhand
//! ungepruefter Werte (`ATC-AI-LLM-002` Abschnitt 11).

use super::sha256::{digest, Sha256};
use super::{
    read, write, ReadError, ReadOptions, TrustAnchor, WriteError, ENTRY_LEN, E_DIM0, E_DIM1,
    E_NAME, E_NAME_LEN, E_OFFSET, FLAG_TIED, HEADER_LEN, MAX_TENSOR_BYTES, OFF_DATA_BYTES,
    OFF_DIGEST, OFF_DTYPE, OFF_D_MODEL, OFF_FLAGS, OFF_LAYOUT, OFF_NORM_EPS, OFF_RESERVED,
    OFF_ROPE_BASE, OFF_TENSOR_COUNT, OFF_VERSION,
};
use crate::config::Config;
use crate::forward::forward;
use crate::params::param_count;
use crate::weights::Weights;
use atc_aurora_tensor::Budget;

fn small(tied: bool) -> Config {
    Config {
        d_model: 16,
        n_layers: 2,
        n_heads: 4,
        n_kv_heads: 2,
        d_head: 4,
        d_ff: 32,
        vocab: 32,
        max_seq: 8,
        tied_embedding: tied,
    }
}

fn options() -> ReadOptions {
    ReadOptions::lenient(1 << 26)
}

fn budget() -> Budget {
    Budget::new(1 << 26)
}

/// Frisches Artefakt plus Digest.
fn artifact(cfg: &Config, seed: u64) -> (Vec<u8>, [u8; 32], Weights) {
    let w = Weights::from_seed(cfg, seed);
    let mut bytes = Vec::new();
    let digest = write(cfg, &w, &mut bytes).expect("Schreiben muss gelingen");
    (bytes, digest, w)
}

/// Setzt ein Feld im Header.
fn put(offset: usize, len: usize, value: u64, bytes: &mut [u8]) {
    let le = value.to_le_bytes();
    bytes[offset..offset + len].copy_from_slice(&le[..len]);
}

fn entry(index: usize, field: usize) -> usize {
    HEADER_LEN + ENTRY_LEN * index + field
}

/// Berechnet den Digest neu, damit ein strukturell gueltiges Artefakt
/// entsteht. Nur fuer Faelle, die *nach* der Strukturpruefung greifen sollen.
fn reseal(bytes: &mut [u8]) {
    let mut hasher = Sha256::new();
    hasher.update(&bytes[..OFF_DIGEST]);
    hasher.update(&[0u8; 32]);
    hasher.update(&bytes[OFF_DIGEST + 32..]);
    let d = hasher.finish();
    bytes[OFF_DIGEST..OFF_DIGEST + 32].copy_from_slice(&d);
}

fn read_err(bytes: &[u8], cfg: Option<&Config>) -> ReadError {
    match read(bytes, TrustAnchor::None, &options(), &mut budget(), cfg) {
        Ok(_) => panic!("Artefakt haette abgelehnt werden muessen"),
        Err(e) => e,
    }
}

// --- Digest selbst ---------------------------------------------------------

#[test]
fn sha256_trifft_die_bekannten_werte() {
    let hex = |d: [u8; 32]| -> String { d.iter().map(|b| format!("{b:02x}")).collect() };
    assert_eq!(
        hex(digest(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(digest(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(digest(
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        )),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    let million = vec![b'a'; 1_000_000];
    assert_eq!(
        hex(digest(&million)),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn sha256_verarbeitet_stueckweise_dasselbe() {
    let data: Vec<u8> = (0..300u32).map(|i| (i % 251) as u8).collect();
    let whole = digest(&data);
    let mut h = Sha256::new();
    for chunk in data.chunks(7) {
        h.update(chunk);
    }
    assert_eq!(h.finish(), whole);
}

// --- B-21: Schreiben, Lesen, Determinismus --------------------------------

#[test]
fn b_21_roundtrip_erhaelt_alle_gewichte() {
    let cfg = small(true);
    let (bytes, digest_in, w) = artifact(&cfg, 42);
    assert_eq!(&bytes[OFF_DIGEST..OFF_DIGEST + 32], &digest_in);

    let a = read(
        &bytes,
        TrustAnchor::ExpectedDigest(digest_in),
        &options(),
        &mut budget(),
        Some(&cfg),
    )
    .expect("gueltiges Artefakt");
    assert!(a.trustworthy());
    assert_eq!(a.config(), &cfg);
    assert_eq!(a.tensors().len(), 2 + 8 * cfg.n_layers);

    let back = a.to_weights().expect("Gewichte lesbar");
    assert_eq!(back.param_count(), w.param_count());
    assert_eq!(back.tok_emb.as_slice(), w.tok_emb.as_slice());
    assert_eq!(back.final_norm.as_slice(), w.final_norm.as_slice());
    for (x, y) in back.blocks.iter().zip(w.blocks.iter()) {
        assert_eq!(x.wq.as_slice(), y.wq.as_slice());
        assert_eq!(x.wgu.as_slice(), y.wgu.as_slice());
    }
}

#[test]
fn b_21_zweiter_lauf_ist_byte_identisch() {
    let cfg = small(true);
    let (a, da, _) = artifact(&cfg, 7);
    let (b, db, _) = artifact(&cfg, 7);
    assert_eq!(a, b);
    assert_eq!(da, db);
    let (c, dc, _) = artifact(&cfg, 8);
    assert_ne!(a, c);
    assert_ne!(da, dc);
}

#[test]
fn b_21_gelesenes_erneut_geschrieben_ist_byte_identisch() {
    let cfg = small(true);
    let (bytes, digest_in, _) = artifact(&cfg, 11);
    let a = read(
        &bytes,
        TrustAnchor::ExpectedDigest(digest_in),
        &options(),
        &mut budget(),
        None,
    )
    .unwrap();
    let w = a.to_weights().unwrap();
    let mut again = Vec::new();
    let digest_out = write(&cfg, &w, &mut again).unwrap();
    assert_eq!(again, bytes, "Kanonizitaet: dieselben Bytes");
    assert_eq!(digest_out, digest_in);
}

#[test]
fn b_21_nicht_gebundene_einbettung_roundtrip() {
    let cfg = small(false);
    let (bytes, digest_in, w) = artifact(&cfg, 5);
    assert!(w.lm_head.is_some());
    assert_eq!(w.param_count(), param_count(&cfg).unwrap());
    let a = read(
        &bytes,
        TrustAnchor::ExpectedDigest(digest_in),
        &options(),
        &mut budget(),
        Some(&cfg),
    )
    .unwrap();
    assert_eq!(a.tensors().len(), 3 + 8 * cfg.n_layers);
    assert_eq!(a.tensors()[0].name(), "tok_emb");
    assert_eq!(a.tensors()[1].name(), "lm_head");
    assert_eq!(a.tensors()[a.tensors().len() - 1].name(), "final_norm");
    let back = a.to_weights().unwrap();
    let head = back.lm_head.clone().expect("lm_head");
    assert_eq!(head.shape(), [cfg.d_model, cfg.vocab]);
    assert_eq!(head.as_slice(), w.lm_head.unwrap().as_slice());

    // Der Vorwaertslauf benutzt den eigenen Kopf.
    let logits = forward(&cfg, &back, &[1, 2, 3], &mut budget()).unwrap();
    assert_eq!(logits.shape(), [3, cfg.vocab]);
}

// --- AT-01 bis AT-08: Laenge, Kennung, Version, Zaehler -------------------

#[test]
fn at_01_datei_im_header_gekuerzt() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    bytes.truncate(50);
    assert_eq!(read_err(&bytes, None), ReadError::Truncated { at: 50 });
}

#[test]
fn at_02_datei_um_ein_byte_verlaengert() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    bytes.push(0);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::TrailingBytes { extra: 1 }
    );
}

#[test]
fn at_03_magic_veraendert() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    bytes[0] = b'X';
    assert_eq!(read_err(&bytes, None), ReadError::BadMagic);
}

#[test]
fn at_04_version_zwei() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_VERSION, 2, 2, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::UnsupportedVersion {
            got: 2,
            expected: 1
        }
    );
}

#[test]
fn at_05_version_null() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_VERSION, 2, 0, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::UnsupportedVersion {
            got: 0,
            expected: 1
        }
    );
}

#[test]
fn at_06_tensorzahl_um_eins_zu_hoch() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let count = (2 + 8 * cfg.n_layers) as u64;
    put(OFF_TENSOR_COUNT, 4, count + 1, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::TensorCountMismatch {
            got: count as usize + 1,
            expected: count as usize
        }
    );
}

#[test]
fn at_07_tensorzahl_riesig() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_TENSOR_COUNT, 4, 1 << 31, &mut bytes);
    assert!(matches!(
        read_err(&bytes, None),
        ReadError::TensorCountMismatch { .. }
    ));
}

#[test]
fn at_08_datenmenge_riesig() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_DATA_BYTES, 8, 1 << 40, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::TooLarge {
            requested: 1 << 40,
            limit: MAX_TENSOR_BYTES
        }
    );
}

// --- AT-09 bis AT-15: Parameter, Flags, Typ, Layout -----------------------

#[test]
fn at_09_d_model_verstellt() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_D_MODEL, 4, 32, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::InvalidArgument { field: "config" }
    );
}

#[test]
fn at_10_rope_base_null() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_ROPE_BASE, 4, 0, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::InvalidArgument { field: "rope_base" }
    );
}

#[test]
fn at_11_norm_eps_null() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_NORM_EPS, 4, 0, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::InvalidArgument { field: "norm_eps" }
    );
}

#[test]
fn at_12_reserved_gesetzt() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_RESERVED, 4, 1, &mut bytes);
    assert_eq!(read_err(&bytes, None), ReadError::ReservedNotZero);
}

#[test]
fn at_13_unbekanntes_flag() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_FLAGS, 2, (FLAG_TIED | 1 << 5) as u64, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::UnknownFlag { bits: 1 << 5 }
    );
}

#[test]
fn at_14_unbekannter_datentyp() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_DTYPE, 4, 1, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::UnsupportedDtype { got: 1 }
    );
}

#[test]
fn at_15_unbekanntes_layout() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(OFF_LAYOUT, 4, 1, &mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::UnsupportedLayout { got: 1 }
    );
}

// --- AT-16 bis AT-22: Tabelle ---------------------------------------------

#[test]
fn at_16_tensoren_vertauscht() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let a = HEADER_LEN + ENTRY_LEN;
    let b = HEADER_LEN + ENTRY_LEN * 2;
    for i in 0..ENTRY_LEN {
        bytes.swap(a + i, b + i);
    }
    reseal(&mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::TensorOrderInvalid { index: 1 }
    );
}

#[test]
fn at_17_offset_verschoben() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(entry(2, E_OFFSET), 8, 4, &mut bytes);
    reseal(&mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::OffsetNotContiguous { index: 2 }
    );
}

#[test]
fn at_18_byte_len_halbiert() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let at = entry(0, 0) + 56;
    let len = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    put(at, 8, len / 2, &mut bytes);
    reseal(&mut bytes);
    assert!(matches!(
        read_err(&bytes, None),
        ReadError::ByteLenMismatch { index: 0, .. }
    ));
}

#[test]
fn at_19_dimension_null() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(entry(0, E_DIM0), 4, 0, &mut bytes);
    reseal(&mut bytes);
    assert_eq!(read_err(&bytes, None), ReadError::ShapeInvalid { index: 0 });
}

#[test]
fn at_20_dimensionen_im_ueberlaufbereich() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(entry(0, E_DIM0), 4, 1 << 20, &mut bytes);
    put(entry(0, E_DIM1), 4, 1 << 20, &mut bytes);
    reseal(&mut bytes);
    assert_eq!(read_err(&bytes, None), ReadError::ShapeInvalid { index: 0 });
}

#[test]
fn at_21_name_mit_grossbuchstaben() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let at = entry(0, E_NAME);
    bytes[at] = b'T';
    reseal(&mut bytes);
    assert_eq!(read_err(&bytes, None), ReadError::InvalidName { index: 0 });
}

#[test]
fn at_22_name_mit_nul_in_der_mitte() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let at = entry(0, E_NAME);
    bytes[at + 3] = 0;
    reseal(&mut bytes);
    assert_eq!(read_err(&bytes, None), ReadError::InvalidName { index: 0 });
}

// --- AT-23 bis AT-27: Werte, Digest, Anker, Architektur, Budget -----------

#[test]
fn at_23_nan_im_strengen_modus() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let first_value = HEADER_LEN + ENTRY_LEN * (2 + 8 * cfg.n_layers) + 16;
    bytes[first_value..first_value + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    reseal(&mut bytes);
    let strict = options().with_finite_check();
    assert!(matches!(
        read(&bytes, TrustAnchor::None, &strict, &mut budget(), None),
        Err(ReadError::ValueNotFinite { index: 0 })
    ));
    // Ohne Endlichkeitspruefung wird die Datei gelesen; das ist eine bewusste
    // Entscheidung des Aufrufers, keine stille Zusage des Lesers.
    assert!(read(&bytes, TrustAnchor::None, &options(), &mut budget(), None).is_ok());
}

#[test]
fn at_24_gewicht_veraendert_ohne_digest() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let data_start = HEADER_LEN + ENTRY_LEN * (2 + 8 * cfg.n_layers);
    bytes[data_start] ^= 0xFF;
    assert!(matches!(
        read_err(&bytes, None),
        ReadError::DigestMismatch { .. }
    ));
}

#[test]
fn at_25_ohne_anker_im_strengen_modus() {
    let cfg = small(true);
    let (bytes, _, _) = artifact(&cfg, 1);
    let strict = ReadOptions::strict(1 << 26);
    assert!(matches!(
        read(&bytes, TrustAnchor::None, &strict, &mut budget(), None),
        Err(ReadError::TrustAnchorMissing)
    ));
    // Und ohne Anker im weiten Modus: lesbar, aber nicht vertrauenswuerdig.
    let a = read(&bytes, TrustAnchor::None, &options(), &mut budget(), None).unwrap();
    assert!(!a.trustworthy());
}

#[test]
fn at_26_architektur_weicht_ab() {
    let cfg = small(true);
    let (bytes, digest_in, _) = artifact(&cfg, 1);
    let mut other = cfg;
    other.d_ff = 64;
    assert!(matches!(
        read(
            &bytes,
            TrustAnchor::ExpectedDigest(digest_in),
            &options(),
            &mut budget(),
            Some(&other),
        ),
        Err(ReadError::ArchMismatch { field: "d_ff" })
    ));
}

#[test]
fn at_27_budget_zu_klein() {
    let cfg = small(true);
    let (bytes, _, _) = artifact(&cfg, 1);
    let mut small_budget = Budget::new(16);
    assert!(matches!(
        read(
            &bytes,
            TrustAnchor::None,
            &options(),
            &mut small_budget,
            None
        ),
        Err(ReadError::BudgetExceeded { .. })
    ));
}

// --- AT-28: Bitmutationen ------------------------------------------------

#[test]
fn at_28_tausend_bitmutationen_ohne_panik() {
    let cfg = small(true);
    let (base, _, _) = artifact(&cfg, 3);
    let mut rng = crate::weights::SplitMix64::new(2026);
    for _ in 0..1000 {
        let mut bytes = base.clone();
        let pos = (rng.next_u64() as usize) % bytes.len();
        let bit = (rng.next_u64() % 8) as u32;
        bytes[pos] ^= 1 << bit;
        if let Ok(a) = read(&bytes, TrustAnchor::None, &options(), &mut budget(), None) {
            // Nur moeglich, wenn der Digest neu passt; dann muss der Inhalt
            // auch strukturell in Ordnung sein.
            assert_eq!(a.tensors().len(), 2 + 8 * cfg.n_layers);
        }
    }
}

// --- AT-29 bis AT-35: nicht gebundene Einbettung, Namen, Offsets, Grenzen --

#[test]
fn at_29_untied_ohne_kopf() {
    let cfg = small(false);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    // Flag auf "gebunden" setzen, ohne die Tabelle zu aendern.
    put(OFF_FLAGS, 2, FLAG_TIED as u64, &mut bytes);
    reseal(&mut bytes);
    assert!(matches!(
        read_err(&bytes, None),
        ReadError::TensorCountMismatch { .. }
    ));
}

#[test]
fn at_30_kopf_an_falscher_position() {
    let cfg = small(false);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let first = HEADER_LEN + ENTRY_LEN;
    let second = HEADER_LEN + ENTRY_LEN * 2;
    for i in 0..ENTRY_LEN {
        bytes.swap(first + i, second + i);
    }
    reseal(&mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::TensorOrderInvalid { index: 1 }
    );
}

#[test]
fn at_31_kopf_mit_falscher_form() {
    let cfg = small(false);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    put(entry(1, E_DIM0), 4, cfg.vocab as u64, &mut bytes);
    put(entry(1, E_DIM1), 4, cfg.d_model as u64, &mut bytes);
    reseal(&mut bytes);
    assert_eq!(read_err(&bytes, None), ReadError::ShapeInvalid { index: 1 });
}

#[test]
fn at_32_doppelter_name() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    // Namen des zweiten Eintrags auf den des ersten setzen.
    let first = entry(0, E_NAME);
    let second = entry(1, E_NAME);
    let mut name = [0u8; 32];
    name.copy_from_slice(&bytes[first..first + 32]);
    bytes[second..second + 32].copy_from_slice(&name);
    let len = u32::from_le_bytes(
        bytes[entry(0, E_NAME_LEN)..entry(0, E_NAME_LEN) + 4]
            .try_into()
            .unwrap(),
    );
    put(entry(1, E_NAME_LEN), 4, len as u64, &mut bytes);
    reseal(&mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::TensorOrderInvalid { index: 1 }
    );
}

#[test]
fn at_33_offset_nahe_2_hoch_63() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    let last = (2 + 8 * cfg.n_layers) - 1;
    put(entry(last, E_OFFSET), 8, 1 << 63, &mut bytes);
    reseal(&mut bytes);
    assert_eq!(
        read_err(&bytes, None),
        ReadError::OffsetNotContiguous { index: last }
    );
}

#[test]
fn at_34_max_bytes_vor_dem_hashen() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    // Digest zusaetzlich zerstoeren: waere er zuerst geprueft worden, kaeme
    // DigestMismatch. Erwartet wird TooLarge, also die Grenze zuerst.
    bytes[OFF_DIGEST] ^= 0xFF;
    let tight = ReadOptions::lenient(bytes.len() - 1);
    assert!(matches!(
        read(&bytes, TrustAnchor::None, &tight, &mut budget(), None),
        Err(ReadError::TooLarge { .. })
    ));
}

#[test]
fn at_35_letztes_byte_entfernt() {
    let cfg = small(true);
    let (mut bytes, _, _) = artifact(&cfg, 1);
    bytes.pop();
    assert!(matches!(
        read_err(&bytes, None),
        ReadError::LengthMismatch { .. }
    ));
}

// --- weitere Pfade -------------------------------------------------------

#[test]
fn schreiben_lehnt_unstimmige_gewichte_ab() {
    let cfg = small(true);
    let w = Weights::from_seed(&cfg, 1);
    let mut other = cfg;
    other.d_ff = 64;
    let mut out = Vec::new();
    assert_eq!(
        write(&other, &w, &mut out),
        Err(WriteError::WeightMismatch { what: "block" })
    );
}

#[test]
fn schreiben_lehnt_nicht_endliche_gewichte_ab() {
    let cfg = small(true);
    let mut w = Weights::from_seed(&cfg, 1);
    let mut data = w.tok_emb.as_slice().to_vec();
    data[3] = f32::INFINITY;
    w.tok_emb = atc_aurora_tensor::Tensor::new(vec![cfg.vocab, cfg.d_model], data).unwrap();
    let mut out = Vec::new();
    assert_eq!(
        write(&cfg, &w, &mut out),
        Err(WriteError::ValueNotFinite { index: 0 })
    );
}

#[test]
fn header_ist_104_byte_und_lueckenlos() {
    // Der Befund F-01 der Gate-Pruefung: die Feldsumme muss die Headerlaenge
    // ergeben, sonst waere die Tabelle um die Differenz verschoben.
    let sum = 4 + 2 + 2 + 4 * 11 + 4 + 4 + 4 + 8 + 32;
    assert_eq!(sum, HEADER_LEN);
    let cfg = small(true);
    let (bytes, _, _) = artifact(&cfg, 1);
    let count = 2 + 8 * cfg.n_layers;
    assert_eq!(
        bytes.len(),
        HEADER_LEN + ENTRY_LEN * count + (bytes.len() - HEADER_LEN - ENTRY_LEN * count)
    );
    assert_eq!(HEADER_LEN % 8, 0, "Datenbereich bleibt 8-Byte-gebunden");
}

#[test]
fn artefakt_der_referenz_traegt_die_paramterzahl() {
    let cfg = Config::ref1();
    let w = Weights::from_seed(&cfg, 1);
    let mut bytes = Vec::new();
    let d = write(&cfg, &w, &mut bytes).unwrap();
    let a = read(
        &bytes,
        TrustAnchor::ExpectedDigest(d),
        &ReadOptions::strict(1 << 31),
        &mut Budget::new(1 << 31),
        Some(&cfg),
    )
    .unwrap();
    assert_eq!(a.config(), &cfg);
    assert_eq!(a.tensors().len(), 2 + 8 * cfg.n_layers);
    let total: u64 = a.tensors().iter().map(|t| t.byte_len()).sum();
    assert_eq!(total, (param_count(&cfg).unwrap() as u64) * 4);
    assert_eq!(
        total,
        u64::from_le_bytes(
            bytes[OFF_DATA_BYTES..OFF_DATA_BYTES + 8]
                .try_into()
                .unwrap()
        )
    );
}

/// Messung der Dekodierkosten, nicht Teil der Suite.
///
/// Aufruf: `cargo test --release -- --ignored --nocapture messung`.
/// Die Zahlen stehen in `docs/measurements/2026-11-11-artifact-decode-cost.md`.
#[test]
#[ignore = "Messung, kein Prueffall"]
fn messung_dekodierkosten_ref1() {
    use std::time::Instant;
    let cfg = Config::ref1();
    let w = Weights::from_seed(&cfg, 1);
    let mut bytes = Vec::new();
    let digest = write(&cfg, &w, &mut bytes).unwrap();
    println!(
        "Datei: {} Byte, Tensoren: {}",
        bytes.len(),
        2 + 8 * cfg.n_layers
    );

    let t0 = Instant::now();
    let a = read(
        &bytes,
        TrustAnchor::ExpectedDigest(digest),
        &ReadOptions::strict(1 << 31),
        &mut Budget::new(1 << 31),
        Some(&cfg),
    )
    .unwrap();
    let t_read = t0.elapsed();
    println!("read (Struktur + Digest + Anker): {t_read:?}");

    let t_h = Instant::now();
    let _ = super::sha256::digest(&bytes);
    println!("nur SHA-256 ueber die ganze Datei: {:?}", t_h.elapsed());

    let t1 = Instant::now();
    let mut elements = 0usize;
    for index in 0..a.tensors().len() {
        elements += a.tensor_f32(index).unwrap().len();
    }
    let t_f32 = t1.elapsed();
    println!("tensor_f32 je Tensor ({elements} Elemente): {t_f32:?}");

    let t2 = Instant::now();
    let back = a.to_weights().unwrap();
    let t_weights = t2.elapsed();
    println!("to_weights (Vollkopie): {t_weights:?}");
    println!("Elemente gleich: {}", back.param_count() == w.param_count());
    println!(
        "Verhaeltnis to_weights/tensor_f32: {:.2}",
        t_weights.as_secs_f64() / t_f32.as_secs_f64()
    );
}

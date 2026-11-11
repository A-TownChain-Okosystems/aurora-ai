//! Pruefungen zu M-05, soweit sie ohne Vortwaertslauf entscheidbar sind.
//!
//! B-18 verlangt eine exakte Parameterzahl. Die Zahl wird hier aus der
//! Architektur abgeleitet und gegen die eingefrorene Konfiguration geprueft.

use super::config::{Config, RefusalReason};
use super::params::{breakdown, param_count, Breakdown, BreakdownError};

/// B-18: die Ableitung fuer die eingefrorene Referenz (K4 Vokabular 4096,
/// K5 d_ff 704, sechs Bloecke, geteilte Einbettung, keine Bias-Terme) ergibt
/// **exakt** den Sollwert 5 868 800, Abweichung 0.
///
/// Der Sollwert ist am 11.11.2026 per Owner-Entscheid A nachgezogen worden
/// (vorher 5 771 776, der Stand vor K5). Die Formel enthaelt seit F1 den Term
/// `+ d` fuer die abschliessende Norm; ohne ihn waere der Wert 5 868 544.
#[test]
fn b_18_parameterzahl_der_referenz_ist_abgeleitet() {
    let cfg = Config::ref1();
    let b: Breakdown = breakdown(&cfg).expect("Referenzkonfiguration ist zulaessig");
    assert_eq!(b.embedding, 1_048_576, "4096 mal 256");
    assert_eq!(b.output_projection, 0, "geteilte Einbettung");
    assert_eq!(b.attention, 1_572_864, "6 Bloecke mal 1024 mal 256");
    assert_eq!(b.mlp, 3_244_032, "6 Bloecke mal 2112 mal 256");
    assert_eq!(b.block_norms, 3_072, "6 Bloecke mal 2 mal 256");
    assert_eq!(b.final_norm, 256);
    assert_eq!(param_count(&cfg), Ok(5_868_800));

    // Der Sollwert ist der Sollwert: Abweichung 0, nicht "ungefaehr".
    let sollwert = 5_868_800usize;
    assert_eq!(param_count(&cfg).unwrap(), sollwert);
}

/// Ohne die abschliessende Norm sind es 256 weniger. Beide Zahlen sind
/// ableitbar; welche gemeint ist, muss der Vertrag festlegen, nicht der Zufall.
#[test]
fn b_18_unterschied_mit_und_ohne_abschliessende_norm_ist_256() {
    let cfg = Config::ref1();
    let b = breakdown(&cfg).unwrap();
    let ohne_final = b.embedding + b.output_projection + b.attention + b.mlp + b.block_norms;
    assert_eq!(ohne_final, 5_868_544);
    assert_eq!(ohne_final + b.final_norm, 5_868_800);
}

/// Drei Zahlen, die nicht verwechselt werden duerfen, und ihre Herkunft.
///
/// 5 771 776 ist der Stand vor K5 (`d_ff` 683, ohne abschliessende Norm) und
/// seit dem Owner-Entscheid A **keine gueltige Anforderung** mehr. Der Test
/// bleibt trotzdem stehen: er haelt die Herkunft der Differenz fest, damit die
/// alte Zahl nicht als Rechenfehler zurueckkommt. 97 024 = 96 768 (K5) + 256 (F1).
#[test]
fn b_18_historischer_stand_ist_dokumentiert() {
    let abgeleitet = param_count(&Config::ref1()).unwrap();
    let vor_k5 = 5_771_776usize;
    let nach_k5_ohne_final = 5_868_544usize;
    assert_eq!(abgeleitet, 5_868_800);
    assert_eq!(abgeleitet - vor_k5, 97_024, "K5 96 768 plus F1 256");
    assert_eq!(abgeleitet - 256, nach_k5_ohne_final);
    // K5 allein, ohne abschliessende Norm: 3 * d * (704 - 683) * L.
    let k5 = 3 * 256 * (704 - 683) * 6;
    assert_eq!(k5, 96_768);
    assert_eq!(vor_k5 + k5 + 256, abgeleitet);
}

/// Nicht geteilte Einbettung verdoppelt die Einbettung und sonst nichts.
#[test]
fn b_18_nicht_geteilte_einbettung_addiert_genau_eine_einbettung() {
    let mut cfg = Config::ref1();
    let geteilt = param_count(&cfg).unwrap();
    cfg.tied_embedding = false;
    assert_eq!(param_count(&cfg).unwrap(), geteilt + 1_048_576);
}

/// Kleinere Breite muss kleiner werden, nicht gleich bleiben.
#[test]
fn b_18_halbierte_breite_halbiert_nicht_die_parameterzahl() {
    let mut cfg = Config::ref1();
    cfg.d_model = 128;
    cfg.d_head = 16;
    cfg.d_ff = 352;
    let klein = param_count(&cfg).unwrap();
    assert!(klein < 5_868_800);
    // Einbettung 4096 mal 128 = 524 288. Je Block 512 Zeilen Aufmerksamkeit
    // (q 128, k 128, v 128, Ausgabe 128) mal 128 Breite mal 6 Bloecke = 393 216,
    // MLP 1056 Zeilen mal 128 mal 6 = 811 008, Normen 1 536, abschliessend 128.
    assert_eq!(klein, 524_288 + 393_216 + 811_008 + 1_536 + 128);
}

/// Unzulaessige Konfigurationen werden schon bei der Zaehlung abgelehnt.
#[test]
fn b_18_unzulaessige_konfiguration_wird_abgelehnt() {
    let mut cfg = Config::ref1();
    cfg.d_model = 0;
    assert_eq!(
        param_count(&cfg),
        Err(BreakdownError::Invalid(RefusalReason::ZeroDimension))
    );

    let mut cfg = Config::ref1();
    cfg.d_head = 16;
    assert_eq!(
        param_count(&cfg),
        Err(BreakdownError::Invalid(RefusalReason::HeadDimMismatch))
    );

    let mut cfg = Config::ref1();
    cfg.n_kv_heads = 3;
    assert_eq!(
        param_count(&cfg),
        Err(BreakdownError::Invalid(RefusalReason::KvHeadMismatch))
    );
}

/// GQA verkleinert die Schluessel- und Wertprojektion, nicht die Abfrage.
#[test]
fn b_18_gqa_verkleinert_nur_schluessel_und_wert() {
    let mut cfg = Config::ref1();
    let mha = breakdown(&cfg).unwrap().attention;
    cfg.n_kv_heads = 2;
    let gqa = breakdown(&cfg).unwrap().attention;
    // Je Block: q 256 bleibt, kv faellt von 512 auf 128, Ausgabe 256 bleibt.
    assert_eq!(mha - gqa, 6 * (512 - 128) * 256);
}

/// Die Zahl ist reproduzierbar: zweimal rechnen ergibt dasselbe.
#[test]
fn b_18_zaehlung_ist_reproduzierbar() {
    let a = param_count(&Config::ref1()).unwrap();
    let b = param_count(&Config::ref1()).unwrap();
    assert_eq!(a, b);
}

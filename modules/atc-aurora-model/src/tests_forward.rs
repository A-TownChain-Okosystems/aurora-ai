//! Tests des Vorwaertslaufs: B-19 (Kausalmaske) und B-20 (512 Token).
//!
//! Die Pruefungen laufen auf zwei Groessen: einer kleinen Konfiguration, damit
//! Grenzfaelle in Millisekunden pruefbar bleiben, und der eingefrorenen Referenz
//! fuer B-20. Beide benutzen dieselbe Funktion, es gibt keinen zweiten Pfad.

use crate::config::{Config, RefusalReason};
use crate::forward::{forward, ForwardError};
use crate::params::param_count;
use crate::weights::Weights;
use atc_aurora_tensor::Budget;

fn small() -> Config {
    Config {
        d_model: 32,
        n_layers: 2,
        n_heads: 4,
        n_kv_heads: 4,
        d_head: 8,
        d_ff: 64,
        vocab: 64,
        max_seq: 32,
        tied_embedding: true,
    }
}

fn budget() -> Budget {
    Budget::new(1 << 30)
}

/// B-19: eine Aenderung an Position `t` darf Positionen `< t` nicht bewegen.
#[test]
fn b_19_aenderung_an_position_t_wirkt_nur_ab_t() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 7);
    let base: Vec<u32> = (0..8).collect();
    let mut changed = base.clone();
    let t = 5;
    changed[t] = 63;

    let a = forward(&cfg, &w, &base, &mut budget()).unwrap();
    let b = forward(&cfg, &w, &changed, &mut budget()).unwrap();
    assert_eq!(a.shape(), [8, cfg.vocab]);
    assert_eq!(b.shape(), [8, cfg.vocab]);

    let (av, bv) = (a.as_slice(), b.as_slice());
    for pos in 0..8 {
        let row_a = &av[pos * cfg.vocab..(pos + 1) * cfg.vocab];
        let row_b = &bv[pos * cfg.vocab..(pos + 1) * cfg.vocab];
        if pos < t {
            assert_eq!(
                row_a, row_b,
                "Position {pos} haette unveraendert bleiben muessen"
            );
        } else {
            assert_ne!(row_a, row_b, "Position {pos} haette sich aendern muessen");
        }
    }
}

/// B-19, zweite Richtung: die Maske ist auch dann kausal, wenn nur die letzte
/// Position sich aendert; keine Position davor darf sich bewegen.
#[test]
fn b_19_aenderung_der_letzten_position_laesst_alles_davor_unberuehrt() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 11);
    let base: Vec<u32> = vec![3, 1, 4, 1, 5, 9];
    let mut changed = base.clone();
    let last = changed.len() - 1;
    changed[last] = 42;

    let a = forward(&cfg, &w, &base, &mut budget()).unwrap();
    let b = forward(&cfg, &w, &changed, &mut budget()).unwrap();
    let av = a.as_slice();
    let bv = b.as_slice();
    let upto = last * cfg.vocab;
    assert_eq!(&av[..upto], &bv[..upto]);
    assert_ne!(
        &av[upto..(last + 1) * cfg.vocab],
        &bv[upto..(last + 1) * cfg.vocab]
    );
}

/// B-20: Vorwaertslauf ueber 512 Token, Form `(512, 4096)`, alle Werte endlich.
#[test]
fn b_20_vorwaertslauf_ueber_512_token() {
    let cfg = Config::ref1();
    let w = Weights::from_seed(&cfg, 1);
    let tokens: Vec<u32> = (0..cfg.max_seq as u32)
        .map(|i| (i * 37) % cfg.vocab as u32)
        .collect();
    assert_eq!(tokens.len(), 512);

    let logits = forward(&cfg, &w, &tokens, &mut budget()).unwrap();
    assert_eq!(logits.shape(), [512, 4096]);
    assert_eq!(logits.size(), 512 * 4096);
    let bad = logits.as_slice().iter().filter(|v| !v.is_finite()).count();
    assert_eq!(bad, 0, "{bad} nicht endliche Logits");
    assert!(logits.as_slice().iter().any(|v| *v != 0.0));
}

/// Derselbe Startwert ergibt denselben Lauf, Zeichen fuer Zeichen.
#[test]
fn vorwaertslauf_ist_reproduzierbar() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 3);
    let tokens: Vec<u32> = vec![1, 2, 3, 4, 5];
    let a = forward(&cfg, &w, &tokens, &mut budget()).unwrap();
    let b = forward(&cfg, &w, &tokens, &mut budget()).unwrap();
    assert_eq!(a.as_slice(), b.as_slice());

    let other = Weights::from_seed(&cfg, 4);
    let c = forward(&cfg, &other, &tokens, &mut budget()).unwrap();
    assert_ne!(a.as_slice(), c.as_slice());
}

/// Die Gewichte zaehlen genau so viele Elemente, wie `params.rs` ableitet.
#[test]
fn gewichte_entsprechen_dem_parameterhaushalt() {
    for cfg in [small(), Config::ref1()] {
        let w = Weights::from_seed(&cfg, 5);
        assert_eq!(w.param_count(), param_count(&cfg).unwrap());
    }
}

/// Grenzfaelle werden typisiert abgelehnt, nicht als Panik oder NaN.
#[test]
fn vorwaertslauf_lehnt_ungueltige_eingaben_ab() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 9);
    assert_eq!(
        forward(&cfg, &w, &[], &mut budget()),
        Err(ForwardError::EmptySequence)
    );
    let too_long: Vec<u32> = vec![0; cfg.max_seq + 1];
    assert_eq!(
        forward(&cfg, &w, &too_long, &mut budget()),
        Err(ForwardError::SequenceTooLong {
            len: too_long.len(),
            max: cfg.max_seq
        })
    );
    assert_eq!(
        forward(&cfg, &w, &[cfg.vocab as u32], &mut budget()),
        Err(ForwardError::TokenOutOfRange {
            token: cfg.vocab as u32,
            vocab: cfg.vocab
        })
    );

    let mut wrong = small();
    wrong.n_layers = 1;
    assert_eq!(
        forward(&wrong, &w, &[1], &mut budget()),
        Err(ForwardError::WeightMismatch { what: "blocks" })
    );
    assert_eq!(
        forward(&cfg, &w, &[1], &mut Budget::new(64)),
        Err(ForwardError::Tensor(
            atc_aurora_tensor::TensorError::BudgetExceeded {
                requested: 128,
                limit: 64
            }
        ))
    );
}

/// Die Konfiguration wird vor dem Rechnen geprueft.
#[test]
fn unzulaessige_konfiguration_wird_abgelehnt() {
    let mut cfg = small();
    cfg.n_kv_heads = 3;
    let w = Weights::from_seed(&small(), 1);
    assert!(cfg.validate().is_err());
    assert_eq!(cfg.validate(), Err(RefusalReason::KvHeadMismatch));
    assert_eq!(
        forward(&cfg, &w, &[1, 2], &mut budget()),
        Err(ForwardError::WeightMismatch { what: "config" })
    );
}

/// Strukturprobe der Maske: bei Gleichstand der Punkte ist die Attention das
/// Mittel ueber alle erlaubten Positionen `0..=i`, nie ueber spaetere.
#[test]
fn attention_mittelt_ueber_erlaubte_positionen() {
    let cfg = Config {
        d_model: 4,
        n_layers: 1,
        n_heads: 1,
        n_kv_heads: 1,
        d_head: 2,
        d_ff: 8,
        vocab: 8,
        max_seq: 8,
        tied_embedding: true,
    };
    let t = 3;
    let q = atc_aurora_tensor::Tensor::new(vec![t, 2], vec![0.0; t * 2]).unwrap();
    let k = atc_aurora_tensor::Tensor::new(vec![t, 2], vec![0.0; t * 2]).unwrap();
    let v = atc_aurora_tensor::Tensor::new(vec![t, 2], vec![1.0, 0.0, 0.0, 1.0, 4.0, 4.0]).unwrap();

    let out = crate::forward::attention(&cfg, &q, &k, &v, &mut budget()).unwrap();
    assert_eq!(out.shape(), [t, 2]);
    // Position 0 sieht nur j = 0.
    assert_eq!(&out.as_slice()[0..2], &[1.0, 0.0]);
    // Position 1 sieht j = 0 und j = 1 zu gleichen Teilen.
    for (got, want) in out.as_slice()[2..4].iter().zip([0.5f32, 0.5]) {
        assert!((got - want).abs() < 1e-6, "Position 1: {got} statt {want}");
    }
    // Position 2 sieht alle drei.
    for (got, want) in out.as_slice()[4..6].iter().zip([5.0 / 3.0, 5.0 / 3.0]) {
        assert!((got - want).abs() < 1e-6, "Position 2: {got} statt {want}");
    }
}

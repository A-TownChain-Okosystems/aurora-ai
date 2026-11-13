// atc-aurora-train — Tests der Trainingskette (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Geprueft wird die Kette, nicht die Sprachqualitaet: der Verlust muss sinken,
// ein Schritt muss deterministisch sein, ein Pruefpunkt muss bytegleich
// zurueckkommen, und ein veraenderter Pruefpunkt muss abgelehnt werden. Die
// Konfigurationen sind klein, damit die Faelle in Sekunden laufen; keine der
// Pruefungen behauptet etwas ueber ein trainiertes Modell.

use atc_aurora_model::artifact::ReadError;
use atc_aurora_model::config::Config;
use atc_aurora_model::weights::Weights;

use crate::eval::evaluate;
use crate::gradients::Gradients;
use crate::optim::{Adam, HyperParams};
use crate::registry::{restore, Registry};
use crate::state::decode;
use crate::trainer::Trainer;
use crate::TrainError;

fn tiny() -> Config {
    Config {
        d_model: 8,
        n_layers: 1,
        n_heads: 2,
        n_kv_heads: 2,
        d_head: 4,
        d_ff: 8,
        vocab: 12,
        max_seq: 8,
        tied_embedding: true,
    }
}

fn corpus() -> Vec<Vec<u32>> {
    vec![
        vec![1, 2, 3, 4, 5],
        vec![6, 7, 8, 9, 10],
        vec![2, 3, 4, 5, 6],
        vec![7, 8, 9, 10, 11],
    ]
}

fn trainer(cfg: Config, seed: u64, lr: f32) -> Trainer {
    let weights = Weights::from_seed(&cfg, seed);
    Trainer::new(cfg, weights, HyperParams::adam(lr)).unwrap()
}

// ------------------------------------------------------------- Optimierer

/// Adam auf einer quadratischen Funktion: ein Parameter, Gradient
/// `2 * (w - 1)`. Nach 400 Schritten muss der Wert nahe 1 liegen. Das prueft
/// Moment, Bias-Korrektur und Vorzeichen in einem Lauf.
#[test]
fn p4_adam_minimiert_eine_quadratische_funktion() {
    let cfg = tiny();
    let mut w = Weights::from_seed(&cfg, 3);
    let mut adam = Adam::new(HyperParams::adam(0.05), &w).unwrap();
    let mut grads = Gradients::zeros_like(&w);
    let index = 0usize;

    for _ in 0..400 {
        let tensors = w.tensors();
        let value = tensors[index].as_slice()[0];
        drop(tensors);
        grads.zero();
        let count = grads.get(index).len();
        let g: Vec<f32> = (0..count)
            .map(|i| if i == 0 { 2.0 * (value - 1.0) } else { 0.0 })
            .collect();
        grads.set(index, &g);
        let mut params = w.tensors_mut();
        adam.step(&mut params, &grads).unwrap();
    }
    let tensors = w.tensors();
    let final_value = tensors[index].as_slice()[0];
    assert!(
        (final_value - 1.0).abs() < 0.05,
        "Adam ist nicht konvergiert: {final_value}"
    );
}

/// Ein abgelehnter Schritt darf den Schrittzaehler nicht verschieben: die
/// Pruefung passiert vor der Zustandsaenderung.
#[test]
fn p4_abgelehnter_schritt_verschiebt_den_zaehler_nicht() {
    let cfg = tiny();
    let mut w = Weights::from_seed(&cfg, 3);
    let mut adam = Adam::new(HyperParams::adam(0.01), &w).unwrap();
    let mut grads = Gradients::zeros_like(&w);
    grads.zero();

    {
        let mut params = w.tensors_mut();
        adam.step(&mut params, &grads).unwrap();
    }
    assert_eq!(adam.step_count(), 1);

    // Ein Parameter zu wenig: der Schritt wird vorher abgelehnt.
    let mut few = {
        let mut params = w.tensors_mut();
        params.pop();
        params
    };
    assert!(matches!(
        adam.step(&mut few, &grads),
        Err(TrainError::WeightMismatch { .. })
    ));
    assert_eq!(adam.step_count(), 1);
}

#[test]
fn p4_hyperparameter_werden_abgelehnt() {
    let cfg = tiny();
    let w = Weights::from_seed(&cfg, 1);
    for hp in [
        HyperParams {
            lr: 0.0,
            ..HyperParams::adam(0.1)
        },
        HyperParams {
            lr: f32::NAN,
            ..HyperParams::adam(0.1)
        },
        HyperParams {
            beta1: 1.0,
            ..HyperParams::adam(0.1)
        },
        HyperParams {
            beta2: -0.1,
            ..HyperParams::adam(0.1)
        },
        HyperParams {
            eps: 0.0,
            ..HyperParams::adam(0.1)
        },
        HyperParams {
            weight_decay: -0.1,
            ..HyperParams::adam(0.1)
        },
        HyperParams {
            budget_bytes: 0,
            ..HyperParams::adam(0.1)
        },
    ] {
        assert!(matches!(
            Adam::new(hp, &w),
            Err(TrainError::InvalidHyperparameter { .. })
        ));
    }
}

// --------------------------------------------------------------- Training

/// Der Verlust sinkt auf einem festen Korpus. Geprueft wird der Mittelwert der
/// ersten fuenf gegen die letzten fuenf Schritte, damit ein einzelner
/// Ausreisser den Befund nicht kippt.
#[test]
fn p4_training_senkt_den_verlust() {
    let cfg = tiny();
    let mut t = trainer(cfg, 11, 0.02);
    let losses = t.fit(&corpus(), 2, 60).unwrap();
    assert_eq!(losses.len(), 60);
    for l in &losses {
        assert!(l.is_finite(), "Verlust ist nicht endlich: {l}");
    }
    let head: f32 = losses[..5].iter().sum::<f32>() / 5.0;
    let tail: f32 = losses[55..].iter().sum::<f32>() / 5.0;
    assert!(
        tail < head * 0.85,
        "Verlust faellt nicht: Anfang {head:.4}, Ende {tail:.4}"
    );
    assert_eq!(t.step_count(), 60);
    assert!(t.tokens_seen() > 0);
}

/// Zwei Laeufe mit denselben Gewichten und demselben Korpus ergeben denselben
/// Verlauf. Es gibt keinen Zufallsstrom, der das kippen koennte.
#[test]
fn p4_training_ist_deterministisch() {
    let cfg = tiny();
    let mut a = trainer(cfg, 5, 0.02);
    let mut b = trainer(cfg, 5, 0.02);
    let la = a.fit(&corpus(), 2, 6).unwrap();
    let lb = b.fit(&corpus(), 2, 6).unwrap();
    assert_eq!(la, lb);
}

#[test]
fn p4_ungueltiger_stapel_wird_abgelehnt() {
    let cfg = tiny();
    let mut t = trainer(cfg, 2, 0.02);
    assert_eq!(t.step(&[]).unwrap_err(), TrainError::InvalidBatch);
    assert_eq!(t.step(&[vec![1]]).unwrap_err(), TrainError::InvalidBatch);
    assert_eq!(
        t.step(&[vec![1, 2], vec![3]]).unwrap_err(),
        TrainError::InvalidBatch
    );
    assert_eq!(t.step_count(), 0);
}

/// Begrenzung: die gemeldete Norm ist die Norm **vor** der Begrenzung, und der
/// Schritt laeuft trotzdem durch.
#[test]
fn p4_gradientenbegrenzung_wird_angewendet() {
    let cfg = tiny();
    let weights = Weights::from_seed(&cfg, 8);
    let hp = HyperParams {
        clip_norm: Some(1e-3),
        ..HyperParams::adam(0.01)
    };
    let mut t = Trainer::new(cfg, weights, hp).unwrap();
    let report = t.step(&[vec![1, 2, 3, 4]]).unwrap();
    assert!(report.grad_norm > 1e-3);
    assert_eq!(report.clip_norm, Some(1e-3));
}

// -------------------------------------------------------------- Bewertung

#[test]
fn p4_bewertung_liefert_verlust_perplexitaet_und_trefferquote() {
    let cfg = tiny();
    let t = trainer(cfg, 4, 0.02);
    let report = evaluate(&cfg, t.weights(), &corpus(), 1 << 24).unwrap();
    assert_eq!(report.tokens, 16);
    assert!(report.mean_loss.is_finite() && report.mean_loss > 0.0);
    assert!(report.perplexity.is_finite() && report.perplexity >= 1.0);
    assert!((0.0..=1.0).contains(&report.accuracy));
}

#[test]
fn p4_bewertung_lehnt_leere_und_kurze_faelle_ab() {
    let cfg = tiny();
    let w = Weights::from_seed(&cfg, 4);
    assert_eq!(
        evaluate(&cfg, &w, &[], 1 << 20).unwrap_err(),
        TrainError::InvalidBatch
    );
    assert_eq!(
        evaluate(&cfg, &w, &[vec![1]], 1 << 20).unwrap_err(),
        TrainError::InvalidBatch
    );
}

// -------------------------------------------------------- Pruefpunkte

fn trained(seed: u64) -> Trainer {
    let cfg = tiny();
    let mut t = trainer(cfg, seed, 0.02);
    t.fit(&corpus(), 2, 4).unwrap();
    t
}

/// Pruefpunkt und Rueckkehr: Gewichte bytegleich, Schritt und Momente gleich.
#[test]
fn p4_pruefpunkt_kommt_bytegleich_zurueck() {
    let t = trained(7);
    let cp = t.checkpoint().unwrap();
    assert_eq!(cp.step, 4);
    let (weights, state) = restore(&tiny(), &cp, 1 << 24).unwrap();
    assert_eq!(state.step, 4);
    assert_eq!(state.tokens_seen, t.tokens_seen());
    assert_eq!(state.adam_t, 4);
    for (a, b) in t.weights().tensors().iter().zip(weights.tensors()) {
        assert_eq!(a.as_slice(), b.as_slice());
    }
}

/// Ein fortgesetzter Lauf ist derselbe Lauf: gleiche Gewichte, gleiche
/// Momente — also gleicher naechster Schritt.
#[test]
fn p4_fortgesetzter_lauf_ist_derselbe_lauf() {
    let t0 = trained(13);
    let cp = t0.checkpoint().unwrap();
    let restored = Trainer::from_checkpoint(tiny(), &cp, 1 << 24).unwrap();

    let mut a = t0;
    let mut b = restored;
    let la = a.step(&[vec![1, 2, 3, 4]]).unwrap();
    let lb = b.step(&[vec![1, 2, 3, 4]]).unwrap();
    assert_eq!(la.loss, lb.loss);
    assert_eq!(la.grad_norm, lb.grad_norm);
    assert_eq!(a.step_count(), b.step_count());
    for (x, y) in a.weights().tensors().iter().zip(b.weights().tensors()) {
        assert_eq!(x.as_slice(), y.as_slice());
    }
}

/// Ein veraendertes Modellartefakt ist kein gueltiger Pruefpunkt, auch wenn
/// das Register es so registriert hat: der Anker passt nicht mehr.
#[test]
fn p4_manipuliertes_modell_wird_abgelehnt() {
    let t = trained(17);
    let mut cp = t.checkpoint().unwrap();
    let at = cp.model.len() / 2;
    cp.model[at] ^= 0x01;
    assert!(matches!(
        restore(&tiny(), &cp, 1 << 24),
        Err(TrainError::Read(ReadError::DigestMismatch { .. }))
    ));
}

/// Ein veraenderter Zustand wird abgelehnt, weil seine Pruefsumme nicht mehr
/// passt.
#[test]
fn p4_manipulierter_zustand_wird_abgelehnt() {
    let t = trained(19);
    let mut cp = t.checkpoint().unwrap();
    let at = cp.state.len() / 3;
    cp.state[at] ^= 0x01;
    assert!(matches!(
        restore(&tiny(), &cp, 1 << 24),
        Err(TrainError::CorruptState { what: "digest" })
    ));
}

/// Ein Zustand, der zu anderen Gewichten gehoert, wird abgelehnt: die Zahl der
/// Momentpaare stimmt dann nicht.
#[test]
fn p4_zustand_mit_falscher_parameterzahl_wird_abgelehnt() {
    let t = trained(23);
    let cp = t.checkpoint().unwrap();
    let state = decode(&cp.state, 1).unwrap_err();
    assert_eq!(
        state,
        TrainError::CorruptState {
            what: "param_count"
        }
    );
}

#[test]
fn p4_register_ordnet_nach_schritt() {
    let mut t = trainer(tiny(), 29, 0.02);
    let mut reg = Registry::new();
    for _ in 0..3 {
        t.step(&[vec![1, 2, 3, 4]]).unwrap();
        reg.register(t.checkpoint().unwrap()).unwrap();
    }
    assert_eq!(reg.len(), 3);
    assert_eq!(reg.steps(), vec![1, 2, 3]);
    assert_eq!(reg.latest().unwrap().step, 3);
    assert_eq!(reg.get(2).unwrap().step, 2);
    let dup = t.checkpoint().unwrap();
    assert_eq!(
        reg.register(dup).unwrap_err(),
        TrainError::DuplicateCheckpoint { step: 3 }
    );
}

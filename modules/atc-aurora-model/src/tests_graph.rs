//! Tests des differenzierbaren Vorwaertslaufs (P4).
//!
//! Der Graph ist nur dann eine Ableitung des Modells, wenn er erstens dieselben
//! Logits liefert wie `forward` und zweitens die Gradienten numerisch stimmen.
//! Beides wird hier geprueft: Gleichheit gegen den unabhaengigen Vorwaertspfad,
//! Gradient gegen die zentrale Finitedifferenz der Verlustfunktion.

use crate::config::Config;
use crate::decode::generate;
use crate::forward::{forward, ForwardError};
use crate::graph::forward_graph;
use crate::weights::Weights;
use atc_aurora_tensor::{Budget, Budget as B};

fn small() -> Config {
    Config {
        d_model: 16,
        n_layers: 2,
        n_heads: 4,
        n_kv_heads: 2,
        d_head: 4,
        d_ff: 24,
        vocab: 32,
        max_seq: 16,
        tied_embedding: true,
    }
}

fn budget() -> Budget {
    Budget::new(1 << 26)
}

/// P4-1: derselbe Baum, dieselben Gewichte — die Logits stimmen mit `forward`
/// ueberein. Abweichung nur aus der Summationsreihenfolge der kopfweisen
/// Ausgabeprojektion, deshalb relativ und mit Schranke.
#[test]
fn p4_graph_logits_stimmen_mit_forward_ueberein() {
    for tied in [true, false] {
        let mut cfg = small();
        cfg.tied_embedding = tied;
        let w = Weights::from_seed(&cfg, 5);
        let tokens: Vec<u32> = vec![3, 1, 4, 1, 5];

        let reference = forward(&cfg, &w, &tokens, &mut budget()).unwrap();
        let g = forward_graph(&cfg, &w, &tokens, &mut budget()).unwrap();
        let got = g.graph.value(g.logits).unwrap();
        assert_eq!(got.shape(), reference.shape());

        let (a, b) = (reference.as_slice(), got.as_slice());
        let mut worst = 0.0f32;
        for (x, y) in a.iter().zip(b) {
            assert!(x.is_finite() && y.is_finite());
            let d = (x - y).abs() / x.abs().max(1e-3);
            if d > worst {
                worst = d;
            }
        }
        assert!(
            worst < 1e-4,
            "tied={tied}: groesste relative Abweichung {worst:.3e}"
        );
    }
}

/// P4-2: die Parameterknoten stehen in der kanonischen Tensorfolge. Nur dann
/// ist der Gradientenindex der Parameterindex.
#[test]
fn p4_graph_parameterreihenfolge_entspricht_gewichten() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 9);
    let g = forward_graph(&cfg, &w, &[1, 2, 3], &mut budget()).unwrap();
    let tensors = w.tensors();
    assert_eq!(g.params.len(), tensors.len());
    for (i, (node, tensor)) in g.params.iter().zip(tensors).enumerate() {
        assert_eq!(
            g.graph.value(*node).unwrap().shape(),
            tensor.shape(),
            "Parameter {i} hat eine andere Form"
        );
    }
}

/// P4-3: der Rueckwaertspass erreicht jeden Parameter, alle Werte sind endlich,
/// und mindestens ein Gradient ist nicht null. Ein Gradient, der nirgends
/// ankommt, waere ein Graph, der nur so aussieht wie das Modell.
#[test]
fn p4_graph_gradienten_erreichen_jeden_parameter() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 3);
    let tokens: Vec<u32> = vec![2, 7, 1, 9];
    let targets: Vec<u32> = vec![7, 1, 9, 4];
    let mut b = budget();
    let mut g = forward_graph(&cfg, &w, &tokens, &mut b).unwrap();
    let loss = g.graph.cross_entropy(g.logits, &targets, &mut b).unwrap();
    g.graph.backward(loss, None, &mut b).unwrap();

    let mut nonzero = 0usize;
    for (i, node) in g.params.iter().enumerate() {
        let grad = g
            .graph
            .grad(*node)
            .unwrap_or_else(|| panic!("Parameter {i} ohne Gradient"));
        assert_eq!(grad.shape(), g.graph.value(*node).unwrap().shape());
        for v in grad.as_slice() {
            assert!(
                v.is_finite(),
                "Parameter {i} hat einen nicht endlichen Gradienten"
            );
        }
        if grad.as_slice().iter().any(|v| *v != 0.0) {
            nonzero += 1;
        }
    }
    assert_eq!(
        nonzero,
        g.params.len(),
        "nicht jeder Parameter hat einen Beitrag"
    );
}

/// Verlust des Modells in f64, gerechnet ueber den **unabhaengigen**
/// Vorwaertspfad `forward`, nicht ueber den Graphen.
fn loss_f64(cfg: &Config, w: &Weights, tokens: &[u32], targets: &[u32]) -> f64 {
    let logits = forward(cfg, w, tokens, &mut B::new(1 << 26)).unwrap();
    let (n, v) = (logits.shape()[0], logits.shape()[1]);
    let src = logits.as_slice();
    let mut acc = 0.0f64;
    for r in 0..n {
        let row: Vec<f64> = src[r * v..(r + 1) * v].iter().map(|x| *x as f64).collect();
        let mut mx = row[0];
        for &x in &row {
            if x > mx {
                mx = x;
            }
        }
        let mut sum = 0.0f64;
        for &x in &row {
            sum += libm_exp(x - mx);
        }
        acc += (mx + sum.ln()) - row[targets[r] as usize];
    }
    acc
}

fn libm_exp(x: f64) -> f64 {
    // exp ueber die Standardbibliothek; der Test laeuft mit std.
    x.exp()
}

/// P4-4: Richtungsableitung. Der analytische Gradient des Graphen muss in eine
/// beliebige Richtung denselben Wert ergeben wie die zentrale Finitedifferenz
/// des Verlusts aus dem unabhaengigen Vorwaertspfad.
#[test]
fn p4_graph_gradient_gegen_finitedifferenz_der_richtung() {
    let cfg = small();
    let mut w = Weights::from_seed(&cfg, 21);
    let tokens: Vec<u32> = vec![5, 2, 8, 1];
    let targets: Vec<u32> = vec![2, 8, 1, 6];

    // Richtung: nur die Abfrageprojektion des ersten Blocks, deterministisch.
    let mut b = budget();
    let mut g = forward_graph(&cfg, &w, &tokens, &mut b).unwrap();
    let loss = g.graph.cross_entropy(g.logits, &targets, &mut b).unwrap();
    g.graph.backward(loss, None, &mut b).unwrap();
    // Kanonische Folge bei gebundener Einbettung: 0 Einbettung,
    // 1 Attention-Norm, 2 Abfrageprojektion.
    let wq_index = 2usize;
    let grad = g.graph.grad(g.params[wq_index]).unwrap().to_vec();

    let n = grad.len();
    let direction: Vec<f32> = (0..n)
        .map(|i| (((i * 13) % 7) as f32 - 3.0) / 3.0)
        .collect();
    let mut analytic = 0.0f64;
    for (gi, di) in grad.iter().zip(&direction) {
        analytic += *gi as f64 * *di as f64;
    }

    let h = 1e-2f32;
    let base = w.blocks[0].wq.to_vec();
    let plus: Vec<f32> = base
        .iter()
        .zip(&direction)
        .map(|(v, d)| v + h * d)
        .collect();
    let minus: Vec<f32> = base
        .iter()
        .zip(&direction)
        .map(|(v, d)| v - h * d)
        .collect();
    w.blocks[0].wq = atc_aurora_tensor::Tensor::new(w.blocks[0].wq.shape().to_vec(), plus).unwrap();
    let l_plus = loss_f64(&cfg, &w, &tokens, &targets);
    w.blocks[0].wq =
        atc_aurora_tensor::Tensor::new(w.blocks[0].wq.shape().to_vec(), minus).unwrap();
    let l_minus = loss_f64(&cfg, &w, &tokens, &targets);
    let numeric = (l_plus - l_minus) / (2.0 * h as f64);

    assert!(analytic.is_finite() && numeric.is_finite());
    assert!(
        analytic.abs() > 1e-4,
        "Richtungsableitung zu klein zum Pruefen: {analytic:.3e}"
    );
    let rel = ((analytic - numeric) / analytic).abs();
    assert!(
        rel < 5e-2,
        "Richtungsableitung analytisch {analytic:.6e}, numerisch {numeric:.6e}, relativ {rel:.3e}"
    );
}

/// P4-5: der Graph gibt beim Aufraeumen jede Buchung zurueck.
#[test]
fn p4_graph_gibt_budget_zurueck() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 4);
    let mut b = budget();
    let before = b.used();
    let mut g = forward_graph(&cfg, &w, &[1, 2, 3], &mut b).unwrap();
    let loss = g.graph.cross_entropy(g.logits, &[2, 3, 1], &mut b).unwrap();
    g.graph.backward(loss, None, &mut b).unwrap();
    assert!(b.used() > before);
    g.graph.clear(&mut b);
    assert_eq!(b.used(), before, "Budget nach clear nicht zurueckgegeben");
}

/// Fehler eines Aufrufs, ohne den Erfolgsfall formatieren zu muessen.
fn err_of(r: Result<crate::graph::GraphForward, ForwardError>) -> ForwardError {
    match r {
        Ok(_) => panic!("Aufruf haette abgelehnt werden muessen"),
        Err(e) => e,
    }
}

/// P4-6: Grenzen des Graphen sind die Grenzen des Vorwaertslaufs.
#[test]
fn p4_graph_grenzen_werden_abgelehnt() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 1);
    assert_eq!(
        err_of(forward_graph(&cfg, &w, &[], &mut budget())),
        ForwardError::EmptySequence
    );
    let too_long: Vec<u32> = vec![1; cfg.max_seq + 1];
    assert!(matches!(
        err_of(forward_graph(&cfg, &w, &too_long, &mut budget())),
        ForwardError::SequenceTooLong { .. }
    ));
    assert!(matches!(
        err_of(forward_graph(&cfg, &w, &[cfg.vocab as u32], &mut budget())),
        ForwardError::TokenOutOfRange { .. }
    ));
}

/// P4-7: Dekodierung ist reproduzierbar, bleibt im Vokabular und respektiert
/// die Laengengrenze des Kontexts.
#[test]
fn p4_generate_ist_reproduzierbar_und_begrenzt() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 2);
    let a = generate(&cfg, &w, &[1, 2, 3], 5, None, &mut B::new(1 << 26)).unwrap();
    let b = generate(&cfg, &w, &[1, 2, 3], 5, None, &mut B::new(1 << 26)).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.generated, 5);
    assert_eq!(a.tokens.len(), 8);
    assert_eq!(&a.tokens[..3], &[1, 2, 3]);
    for t in &a.tokens[4..] {
        assert!((*t as usize) < cfg.vocab);
    }

    let full: Vec<u32> = vec![0; cfg.max_seq];
    let c = generate(&cfg, &w, &full, 5, None, &mut B::new(1 << 26)).unwrap();
    assert_eq!(c.generated, 0);
    assert_eq!(c.tokens, full);

    assert_eq!(
        generate(&cfg, &w, &[], 1, None, &mut B::new(1 << 26)).unwrap_err(),
        ForwardError::EmptySequence
    );
}

/// P4-8: das Stopp-Token beendet die Erzeugung und ist enthalten.
#[test]
fn p4_generate_haelt_am_stopp_token() {
    let cfg = small();
    let w = Weights::from_seed(&cfg, 6);
    let one = generate(&cfg, &w, &[1, 2], 1, None, &mut B::new(1 << 26)).unwrap();
    let stop = *one.tokens.last().unwrap();
    let out = generate(&cfg, &w, &[1, 2], 8, Some(stop), &mut B::new(1 << 26)).unwrap();
    assert_eq!(out.generated, 1);
    assert_eq!(*out.tokens.last().unwrap(), stop);
}

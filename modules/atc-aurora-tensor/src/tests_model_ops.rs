// atc-aurora-tensor — Graphenoperationen der Modellkette (P4-Erweiterung)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Geprueft werden die drei Operationen, die M-05 fuer einen differenzierbaren
// Vorwaertslauf braucht und die in `AUTOGRAD-CONTRACT.md` als offen standen:
// Einbettungs-Lookup, Spaltenblock und RoPE. Referenz ist wie bei B-09 bis
// B-13 die zentrale Finitedifferenz in f64 mit `h = 1e-5`; sie liegt im Test,
// damit der Rueckwaertspfad nicht gegen sich selbst geprueft wird.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::error::TensorError;
use crate::{Budget, Graph, Tensor};

fn budget() -> Budget {
    Budget::new(1 << 22)
}

fn fd<F: Fn(&[f64]) -> f64>(f: F, x: &[f64], h: f64) -> Vec<f64> {
    let mut g = vec![0.0f64; x.len()];
    for i in 0..x.len() {
        let mut xp = x.to_vec();
        let mut xm = x.to_vec();
        xp[i] += h;
        xm[i] -= h;
        g[i] = (f(&xp) - f(&xm)) / (2.0 * h);
    }
    g
}

fn max_rel(got: &[f32], want: &[f64]) -> f64 {
    let mut worst = 0.0f64;
    for (g, w) in got.iter().zip(want) {
        let g = *g as f64;
        let d = if w.abs() > 1e-6 {
            ((g - w) / w).abs()
        } else {
            (g - w).abs()
        };
        if d > worst {
            worst = d;
        }
    }
    worst
}

fn t2(shape: [usize; 2], data: &[f32]) -> Tensor {
    Tensor::new(vec![shape[0], shape[1]], data.to_vec()).unwrap()
}

// ------------------------------------------------------------ Einbettung

/// Verlust `sum(gather(table)^2)` als f64-Referenz.
fn gather_loss_f64(table: &[f64], tokens: &[usize], d: usize) -> f64 {
    let mut acc = 0.0f64;
    for &tok in tokens {
        for c in 0..d {
            let v = table[tok * d + c];
            acc += v * v;
        }
    }
    acc
}

fn gather_grad(table: &[f32], tokens: &[u32], rows: usize, d: usize) -> (f32, Vec<f32>) {
    let mut b = budget();
    let mut g = Graph::new();
    let t = g.parameter(t2([rows, d], table));
    let e = g.gather(t, tokens, &mut b).unwrap();
    let sq = g.mul(e, e, &mut b).unwrap();
    let l = g.sum(sq, &mut b).unwrap();
    g.backward(l, None, &mut b).unwrap();
    let loss = g.value(l).unwrap().as_slice()[0];
    let grad = g.grad(t).unwrap().to_vec();
    (loss, grad)
}

#[test]
fn p4_gather_gradient_gegen_finitedifferenz() {
    let table: Vec<f32> = vec![
        0.5, -1.0, 2.0, 1.5, 0.25, -0.75, -0.3, 0.8, 1.1, 0.9, -0.2, 0.4,
    ];
    let tokens: Vec<u32> = vec![2, 0, 2, 3];
    let (rows, d) = (4usize, 3usize);
    let (loss, grad) = gather_grad(&table, &tokens, rows, d);

    let tablef: Vec<f64> = table.iter().map(|v| *v as f64).collect();
    let tok_usize: Vec<usize> = tokens.iter().map(|v| *v as usize).collect();
    let want = fd(|x| gather_loss_f64(x, &tok_usize, d), &tablef, 1e-5);
    let e = max_rel(&grad, &want);
    assert!(e < 1e-5, "dTable relative Abweichung {e:.3e}");
    let loss_want = gather_loss_f64(&tablef, &tok_usize, d);
    assert!((loss as f64 - loss_want).abs() / loss_want < 1e-6);
}

/// Eine zweimal gelesene Zeile bekommt den doppelten Beitrag, alle anderen
/// Zeilen exakt null. Das ist die Zusage aus B-12, auf den Lookup angewendet.
#[test]
fn p4_gather_mehrfach_gelesene_zeile_summiert() {
    // Verlust = 2 * sum(tabelle[1]^2), also Gradient 4 * tabelle[1].
    let table: Vec<f32> = vec![0.0, 0.0, 0.3, -0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let tokens: Vec<u32> = vec![1, 1];
    let (_, grad) = gather_grad(&table, &tokens, 5, 2);
    assert_eq!(grad.len(), 10);
    assert!((grad[2] - 4.0 * 0.3).abs() < 1e-6);
    assert!((grad[3] - 4.0 * -0.2).abs() < 1e-6);
    for (i, v) in grad.iter().enumerate() {
        if i != 2 && i != 3 {
            assert_eq!(*v, 0.0, "Zeile {i} haette unberuehrt bleiben muessen");
        }
    }
}

#[test]
fn p4_gather_token_ausserhalb_ist_typisiert() {
    let mut b = budget();
    let mut g = Graph::new();
    let t = g.parameter(t2([2, 3], &[0.0; 6]));
    assert_eq!(
        g.gather(t, &[2], &mut b).unwrap_err(),
        TensorError::InvalidArgument {
            what: "gather: token"
        }
    );
}

// ------------------------------------------------------------ Spaltenblock

fn slice_grad(
    x: &[f32],
    rows: usize,
    cols: usize,
    start: usize,
    len: usize,
    w: &[f32],
) -> Vec<f32> {
    let mut b = budget();
    let mut g = Graph::new();
    let xn = g.parameter(t2([rows, cols], x));
    let s = g.slice_cols(xn, start, len, &mut b).unwrap();
    let wn = g.leaf(t2([rows, len], w));
    let p = g.mul(s, wn, &mut b).unwrap();
    let l = g.sum(p, &mut b).unwrap();
    g.backward(l, None, &mut b).unwrap();
    g.grad(xn).unwrap().to_vec()
}

#[test]
fn p4_slice_cols_gradient_gegen_finitedifferenz() {
    let (rows, cols) = (2usize, 6usize);
    let x: Vec<f32> = (0..rows * cols).map(|i| (i as f32) * 0.25 - 1.0).collect();
    let w = [0.5f32, -1.5, 2.0, 0.75, 1.25, -0.5];
    let (start, len) = (2usize, 3usize);
    let grad = slice_grad(&x, rows, cols, start, len, &w);

    let xf: Vec<f64> = x.iter().map(|v| *v as f64).collect();
    let wf: Vec<f64> = w.iter().map(|v| *v as f64).collect();
    let want = fd(
        |p| {
            let mut acc = 0.0f64;
            for r in 0..rows {
                for c in 0..len {
                    acc += p[r * cols + start + c] * wf[r * len + c];
                }
            }
            acc
        },
        &xf,
        1e-5,
    );
    let e = max_rel(&grad, &want);
    assert!(e < 1e-5, "dX relative Abweichung {e:.3e}");
    for r in 0..rows {
        for c in 0..cols {
            if c < start || c >= start + len {
                assert_eq!(grad[r * cols + c], 0.0);
            }
        }
    }
}

#[test]
fn p4_slice_cols_grenzen_werden_abgelehnt() {
    let mut b = budget();
    let mut g = Graph::new();
    let x = g.parameter(t2([2, 4], &[0.0; 8]));
    assert_eq!(
        g.slice_cols(x, 0, 0, &mut b).unwrap_err(),
        TensorError::InvalidArgument {
            what: "slice_cols: len"
        }
    );
    assert_eq!(
        g.slice_cols(x, 2, 3, &mut b).unwrap_err(),
        TensorError::ShapeMismatch
    );
}

// ------------------------------------------------------------------- RoPE

fn rope_loss_f64(
    x: &[f64],
    cos: &[f64],
    sin: &[f64],
    rows: usize,
    width: usize,
    d_head: usize,
    w: &[f64],
) -> f64 {
    let heads = width / d_head;
    let half = d_head / 2;
    let mut acc = 0.0f64;
    for pos in 0..rows {
        for h in 0..heads {
            let base = pos * width + h * d_head;
            for i in 0..half {
                let c = cos[pos * half + i];
                let s = sin[pos * half + i];
                let a = x[base + 2 * i];
                let b = x[base + 2 * i + 1];
                let ra = a * c - b * s;
                let rb = a * s + b * c;
                acc += ra * w[base + 2 * i] + rb * w[base + 2 * i + 1];
            }
        }
    }
    acc
}

#[test]
fn p4_rope_gradient_gegen_finitedifferenz() {
    let (rows, width, d_head) = (2usize, 8usize, 4usize);
    let x: Vec<f32> = (0..rows * width)
        .map(|i| ((i * 7) % 11) as f32 * 0.2 - 1.0)
        .collect();
    let half = d_head / 2;
    let mut cos = Vec::new();
    let mut sin = Vec::new();
    for pos in 0..rows {
        for i in 0..half {
            let angle = pos as f32 * 0.5 + i as f32 * 0.3;
            cos.push(angle.cos());
            sin.push(angle.sin());
        }
    }
    let w: Vec<f32> = (0..rows * width)
        .map(|i| 0.25 + (i % 5) as f32 * 0.1)
        .collect();

    let mut b = budget();
    let mut g = Graph::new();
    let xn = g.parameter(t2([rows, width], &x));
    let cn = g.leaf(t2([rows, half], &cos));
    let sn = g.leaf(t2([rows, half], &sin));
    let r = g.rope(xn, cn, sn, d_head, &mut b).unwrap();
    let wn = g.leaf(t2([rows, width], &w));
    let p = g.mul(r, wn, &mut b).unwrap();
    let l = g.sum(p, &mut b).unwrap();
    g.backward(l, None, &mut b).unwrap();
    let grad = g.grad(xn).unwrap().to_vec();

    let xf: Vec<f64> = x.iter().map(|v| *v as f64).collect();
    let cf: Vec<f64> = cos.iter().map(|v| *v as f64).collect();
    let sf: Vec<f64> = sin.iter().map(|v| *v as f64).collect();
    let wf: Vec<f64> = w.iter().map(|v| *v as f64).collect();
    let want = fd(
        |p| rope_loss_f64(p, &cf, &sf, rows, width, d_head, &wf),
        &xf,
        1e-5,
    );
    let e = max_rel(&grad, &want);
    assert!(e < 1e-5, "dX relative Abweichung {e:.3e}");

    // `cos` und `sin` sind Konstanten: kein Gradient, keine Gradientenpflicht.
    assert!(g.grad(cn).is_none());
    assert!(g.grad(sn).is_none());
    assert!(!g.requires_grad(cn).unwrap());
}

/// Die Rotation ist eine Drehung: die Paarnorm bleibt erhalten, und eine
/// Drehung um 90 Grad vertauscht die beiden Komponenten mit Vorzeichen.
#[test]
fn p4_rope_ist_eine_drehung() {
    let (rows, d_head) = (1usize, 2usize);
    let x = [3.0f32, 4.0];
    let half = 1usize;
    let cos = [0.0f32];
    let sin = [1.0f32];

    let mut b = budget();
    let mut g = Graph::new();
    let xn = g.parameter(t2([rows, d_head], &x));
    let cn = g.leaf(t2([rows, half], &cos));
    let sn = g.leaf(t2([rows, half], &sin));
    let r = g.rope(xn, cn, sn, d_head, &mut b).unwrap();
    let out = g.value(r).unwrap().as_slice().to_vec();
    assert!((out[0] - -4.0).abs() < 1e-6);
    assert!((out[1] - 3.0).abs() < 1e-6);
    let n_in = x[0] * x[0] + x[1] * x[1];
    let n_out = out[0] * out[0] + out[1] * out[1];
    assert!((n_in - n_out).abs() < 1e-5);
}

#[test]
fn p4_rope_ungerade_kopfbreite_wird_abgelehnt() {
    let mut b = budget();
    let mut g = Graph::new();
    let x = g.parameter(t2([1, 4], &[0.0; 4]));
    let cn = g.leaf(t2([1, 1], &[0.0]));
    let sn = g.leaf(t2([1, 1], &[0.0]));
    assert_eq!(
        g.rope(x, cn, sn, 3, &mut b).unwrap_err(),
        TensorError::InvalidArgument {
            what: "rope: d_head"
        }
    );
}

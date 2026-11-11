// atc-aurora-tensor — Vertragstests Autograd (B-08 bis B-13, M-03)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Die Referenz ist die zentrale Finitedifferenz in f64 mit `h = 1e-5`, wie es
// ATC-AI-LLM-001-A Abschnitt 7.1 verlangt. Sie liegt im Test und nicht im Crate,
// weil das Crate f32 als einzigen Puffer führt: eine Referenz aus dem eigenen
// Vorwärtspfad wäre keine Referenz, sondern eine Selbstbestätigung.
//
// Bei kleinen Beträgen (RMS-Norm mit Werten um 1e-4) wird `h` an die
// Größenordnung angepasst und das am Test vermerkt; `h = 1e-5` wäre dort größer
// als der Funktionswert und würde den geprüften Punkt verschieben.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::error::TensorError;
use crate::{cross_entropy, Budget, Graph, Tensor};

fn budget() -> Budget {
    Budget::new(1 << 22)
}

// ------------------------------------------------------ f64-Referenzrechnung

fn f64_matmul(a: &[f64], m: usize, k: usize, b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0f64; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut acc = 0.0f64;
            for x in 0..k {
                acc += a[i * k + x] * b[x * n + j];
            }
            c[i * n + j] = acc;
        }
    }
    c
}

fn f64_softmax_row(x: &[f64]) -> Vec<f64> {
    let mut mx = x[0];
    for &v in x {
        if v > mx {
            mx = v;
        }
    }
    let mut e = vec![0.0f64; x.len()];
    let mut sum = 0.0f64;
    for i in 0..x.len() {
        e[i] = libm::exp(x[i] - mx);
        sum += e[i];
    }
    for v in e.iter_mut() {
        *v /= sum;
    }
    e
}

fn f64_softmax(x: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let mut out = vec![0.0f64; x.len()];
    for r in 0..rows {
        let s = f64_softmax_row(&x[r * cols..(r + 1) * cols]);
        out[r * cols..(r + 1) * cols].copy_from_slice(&s);
    }
    out
}

fn f64_rms_norm_row(x: &[f64], w: &[f64], eps: f64) -> Vec<f64> {
    let d = x.len();
    let mut acc = 0.0f64;
    for &v in x {
        acc += v * v;
    }
    let inv = 1.0 / (acc / d as f64 + eps).sqrt();
    let mut y = vec![0.0f64; d];
    for i in 0..d {
        y[i] = x[i] * inv * w[i];
    }
    y
}

fn f64_silu(t: f64) -> f64 {
    t / (1.0 + libm::exp(-t))
}

fn f64_cross_entropy(logits: &[f64], n: usize, v: usize, targets: &[usize]) -> f64 {
    let mut loss = 0.0f64;
    for r in 0..n {
        let row = &logits[r * v..(r + 1) * v];
        let mut mx = row[0];
        for &x in row {
            if x > mx {
                mx = x;
            }
        }
        let mut sum = 0.0f64;
        for &x in row {
            sum += libm::exp(x - mx);
        }
        loss += (mx + sum.ln()) - row[targets[r]];
    }
    loss
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

/// Größte relative Abweichung. Beträge unter 1e-6 werden absolut geprüft, weil
/// eine relative Angabe dort nur Rauschen messen würde.
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

fn t1(data: &[f32]) -> Tensor {
    Tensor::from_vec(data.to_vec())
}

// ------------------------------------------------------------ B-08

#[test]
fn b_08_kreuzentropie_gradient_ist_softmax_minus_target() {
    // 3 Zeilen, 4 Klassen, Ziel 0, 2, 3.
    let logits = [
        0.4f32, -1.2, 2.5, 0.1, 1.9, 0.05, -0.7, 0.3, -2.2, 1.1, 0.6, 3.4,
    ];
    let targets: [u32; 3] = [0, 2, 3];

    let mut g = Graph::new();
    let x = g.parameter(t2([3, 4], &logits));
    let loss = g.cross_entropy(x, &targets, &mut budget()).unwrap();
    g.backward(loss, None, &mut budget()).unwrap();

    let grad = g.grad(x).expect("Gradient am Logit-Knoten");

    // Erwartung 1: der Verlust des Graphen ist der Verlust der Einzelfunktion.
    let mut b = budget();
    let (loss_val, grad_val) = cross_entropy(&t2([3, 4], &logits), &targets, &mut b).unwrap();
    assert_eq!(g.value(loss).unwrap().size(), 1);
    assert!((g.value(loss).unwrap().as_slice()[0] - loss_val.as_slice()[0]).abs() < 1e-7);

    // Erwartung 2: Gradient == softmax - onehot, exakt aus der Definition.
    let probs = t2([3, 4], &logits).softmax(&mut b).unwrap();
    for i in 0..12 {
        let row = i / 4;
        let col = i % 4;
        let y = if col as u32 == targets[row] {
            1.0f32
        } else {
            0.0
        };
        let want = probs.as_slice()[i] - y;
        let got = grad.as_slice()[i];
        assert!(
            (got - want).abs() <= 1e-7 * want.abs().max(1.0),
            "Index {i}: got {got}, want {want}"
        );
        // und identisch zum Gradienten der Einzelfunktion
        assert!((got - grad_val.as_slice()[i]).abs() < 1e-7);
    }
}

#[test]
fn b_08_zielwahrscheinlichkeit_null_kein_nan_kein_inf() {
    // Abstand 2000 zwischen Ziel- und Höchstlogit: exp(-2000) rundet in f32 auf
    // 0. Die Zielwahrscheinlichkeit ist dann exakt 0, der Verlust muss trotzdem
    // endlich sein (2000,0) und der Gradient -1.
    let logits = t2([1, 2], &[1000.0, -1000.0]);
    let targets: [u32; 1] = [1];
    let mut b = budget();
    let (loss, grad) = cross_entropy(&logits, &targets, &mut b).unwrap();

    let l = loss.as_slice()[0];
    assert!(l.is_finite(), "Verlust nicht endlich: {l}");
    assert!((l - 2000.0).abs() < 1e-3, "Verlust {l}, erwartet 2000");
    for &v in grad.as_slice() {
        assert!(v.is_finite(), "Gradient nicht endlich: {v}");
    }
    // softmax ist [1, 0], Ziel ist Klasse 1 => Gradient p - onehot, also +1 auf
    // der Klasse 0 und -1 auf der Zielklasse.
    assert!((grad.as_slice()[0] - 1.0).abs() < 1e-6);
    assert!((grad.as_slice()[1] + 1.0).abs() < 1e-6);

    // Derselbe Fall über den Graphen.
    let mut g = Graph::new();
    let x = g.parameter(t2([1, 2], &[1000.0, -1000.0]));
    let node = g.cross_entropy(x, &targets, &mut budget()).unwrap();
    g.backward(node, None, &mut budget()).unwrap();
    let gg = g.grad(x).unwrap();
    for &v in gg.as_slice() {
        assert!(v.is_finite());
    }
    assert!((gg.as_slice()[1] + 1.0).abs() < 1e-6);
}

// ------------------------------------------------------------ B-09

#[test]
fn b_09_matmul_gradient_gegen_finitedifferenz() {
    // (2,3) x (3,2) -> (2,2), danach gewichtete Summe als skalarer Verlust.
    let a: [f32; 6] = [0.37, -1.25, 2.1, 0.9, 0.15, -3.4];
    let bb: [f32; 6] = [1.7, -0.6, 0.45, 2.3, -1.1, 0.75];
    let w: [f32; 4] = [0.5, -1.0, 2.0, 0.25];

    let mut g = Graph::new();
    let na = g.parameter(t2([2, 3], &a));
    let nb = g.parameter(t2([3, 2], &bb));
    let nw = g.leaf(t2([2, 2], &w));
    let c = g.matmul(na, nb, &mut budget()).unwrap();
    let m = g.mul(c, nw, &mut budget()).unwrap();
    let l = g.sum(m, &mut budget()).unwrap();
    g.backward(l, None, &mut budget()).unwrap();

    let af: Vec<f64> = a.iter().map(|&x| x as f64).collect();
    let bf: Vec<f64> = bb.iter().map(|&x| x as f64).collect();
    let wf: Vec<f64> = w.iter().map(|&x| x as f64).collect();

    let loss_a = |x: &[f64]| -> f64 {
        let c = f64_matmul(x, 2, 3, &bf, 2);
        let mut s = 0.0f64;
        for i in 0..4 {
            s += c[i] * wf[i];
        }
        s
    };
    let loss_b = |x: &[f64]| -> f64 {
        let c = f64_matmul(&af, 2, 3, x, 2);
        let mut s = 0.0f64;
        for i in 0..4 {
            s += c[i] * wf[i];
        }
        s
    };

    let want_a = fd(loss_a, &af, 1e-5);
    let want_b = fd(loss_b, &bf, 1e-5);

    let ga = g.grad(na).unwrap();
    let gb = g.grad(nb).unwrap();
    let ea = max_rel(ga.as_slice(), &want_a);
    let eb = max_rel(gb.as_slice(), &want_b);
    assert!(ea < 1e-6, "dA relative Abweichung {ea:.3e}");
    assert!(eb < 1e-6, "dB relative Abweichung {eb:.3e}");

    // Der Verlust des Graphen stimmt mit der f64-Referenz überein.
    let want_loss = loss_a(&af);
    let got_loss = g.value(l).unwrap().as_slice()[0] as f64;
    assert!((got_loss - want_loss).abs() < 1e-6);
}

// ------------------------------------------------------------ B-10

#[test]
fn b_10_kette_softmax_und_kreuzentropie() {
    let logits: [f32; 12] = [
        0.4, -1.2, 2.5, 0.1, 1.9, 0.05, -0.7, 0.3, -2.2, 1.1, 0.6, 3.4,
    ];
    let targets: [u32; 3] = [1, 2, 0];
    let lf: Vec<f64> = logits.iter().map(|&x| x as f64).collect();
    let ti: Vec<usize> = targets.iter().map(|&t| t as usize).collect();

    // (a) Autograd durch Softmax und Kreuzentropie.
    let mut g = Graph::new();
    let x = g.parameter(t2([3, 4], &logits));
    let loss = g.cross_entropy(x, &targets, &mut budget()).unwrap();
    g.backward(loss, None, &mut budget()).unwrap();
    let grad = g.grad(x).unwrap().to_vec();

    // (b) gegen die zentrale Finitedifferenz der zusammengesetzten Funktion.
    let want = fd(|v: &[f64]| f64_cross_entropy(v, 3, 4, &ti), &lf, 1e-5);
    let e = max_rel(&grad, &want);
    assert!(e < 1e-6, "Kette CE relative Abweichung {e:.3e}");

    // (c) Gradient == softmax - onehot, mit der Softmax-Operation des Crates.
    let mut b = budget();
    let probs = t2([3, 4], &logits).softmax(&mut b).unwrap();
    for (i, got) in grad.iter().enumerate() {
        let row = i / 4;
        let col = i % 4;
        let y = if col as u32 == targets[row] {
            1.0f32
        } else {
            0.0
        };
        let want_i = probs.as_slice()[i] - y;
        assert!(
            (got - want_i).abs() < 1e-6,
            "Index {i}: {got} gegen {want_i}"
        );
    }
}

#[test]
fn b_10_softmax_knoten_hat_eigene_ableitung() {
    // Die Kette läuft hier durch einen echten Softmax-Knoten: y = softmax(x),
    // L = sum(y * W). Damit wird die Jacobi-Vektor-Multiplikation der Softmax
    // geprüft, nicht nur die zusammengefasste Form der Kreuzentropie.
    let logits: [f32; 6] = [0.4, -1.2, 2.5, 1.9, 0.05, -0.7];
    let w: [f32; 6] = [0.5, -1.0, 2.0, 0.25, 1.5, -0.75];
    let lf: Vec<f64> = logits.iter().map(|&x| x as f64).collect();
    let wf: Vec<f64> = w.iter().map(|&x| x as f64).collect();

    let mut g = Graph::new();
    let x = g.parameter(t2([2, 3], &logits));
    let nw = g.leaf(t2([2, 3], &w));
    let y = g.softmax(x, &mut budget()).unwrap();
    let p = g.mul(y, nw, &mut budget()).unwrap();
    let l = g.sum(p, &mut budget()).unwrap();
    g.backward(l, None, &mut budget()).unwrap();
    let grad = g.grad(x).unwrap().to_vec();

    let want = fd(
        |v: &[f64]| -> f64 {
            let s = f64_softmax(v, 2, 3);
            let mut acc = 0.0f64;
            for i in 0..6 {
                acc += s[i] * wf[i];
            }
            acc
        },
        &lf,
        1e-5,
    );
    let e = max_rel(&grad, &want);
    assert!(e < 1e-6, "Softmax-JVP relative Abweichung {e:.3e}");

    // Summe der Gradienten je Zeile ist null: Softmax verschiebt sich invariant
    // unter einer konstanten Verschiebung der Logits.
    for r in 0..2 {
        let s: f32 = grad[r * 3..(r + 1) * 3].iter().sum();
        assert!(s.abs() < 1e-6, "Zeilensumme des Gradienten {s} statt 0");
    }
}

// ------------------------------------------------------------ B-11

#[test]
fn b_11_rmsnorm_gradient_bei_normnahe_null() {
    // Werte um 1e-4: mean(x^2) liegt in der Größenordnung von eps = 1e-8.
    let x: [f32; 8] = [
        1.0e-4, -2.0e-4, 3.0e-4, -4.0e-4, 2.5e-4, 1.5e-4, -1.0e-4, 3.5e-4,
    ];
    let w: [f32; 4] = [0.5, -1.5, 2.0, 0.25];
    let gconst: [f32; 8] = [1.0, -0.5, 0.75, 2.0, -1.0, 0.25, 1.5, -0.75];
    let eps = 1e-8f32;

    let xf: Vec<f64> = x.iter().map(|&v| v as f64).collect();
    let wf: Vec<f64> = w.iter().map(|&v| v as f64).collect();
    let gf: Vec<f64> = gconst.iter().map(|&v| v as f64).collect();
    let epsf = eps as f64;

    let mut g = Graph::new();
    let nx = g.parameter(t2([2, 4], &x));
    let nw = g.parameter(t1(&w));
    let ng = g.leaf(t2([2, 4], &gconst));
    let y = g.rms_norm(nx, nw, eps, &mut budget()).unwrap();
    let p = g.mul(y, ng, &mut budget()).unwrap();
    let l = g.sum(p, &mut budget()).unwrap();
    g.backward(l, None, &mut budget()).unwrap();

    let gx = g.grad(nx).unwrap().to_vec();
    let gw = g.grad(nw).unwrap().to_vec();
    for &v in gx.iter().chain(gw.iter()) {
        assert!(v.is_finite(), "Gradient nicht endlich: {v}");
    }

    // Referenz in f64. h wird hier an die Größenordnung angepasst (1e-7 statt
    // 1e-5), weil die Eingaben selbst nur 1e-4 groß sind.
    let loss_x = |v: &[f64]| -> f64 {
        let mut acc = 0.0f64;
        for r in 0..2 {
            let row = f64_rms_norm_row(&v[r * 4..(r + 1) * 4], &wf, epsf);
            for c in 0..4 {
                acc += row[c] * gf[r * 4 + c];
            }
        }
        acc
    };
    let loss_w = |v: &[f64]| -> f64 {
        let mut acc = 0.0f64;
        for r in 0..2 {
            let row = f64_rms_norm_row(&xf[r * 4..(r + 1) * 4], v, epsf);
            for c in 0..4 {
                acc += row[c] * gf[r * 4 + c];
            }
        }
        acc
    };
    let want_x = fd(loss_x, &xf, 1e-7);
    let want_w = fd(loss_w, &wf, 1e-7);
    let ex = max_rel(&gx, &want_x);
    let ew = max_rel(&gw, &want_w);
    assert!(ex < 1e-5, "dx relative Abweichung {ex:.3e}");
    assert!(ew < 1e-5, "dw relative Abweichung {ew:.3e}");
}

#[test]
fn b_11_nullzeile_kein_nan_und_kein_division_durch_null() {
    // Zeile 1 ist exakt null. Ohne eps wäre die Wurzel 0 und der Gradient
    // unendlich; mit eps = 1e-8 ist der Faktor 1/sqrt(eps) = 1e4 und endlich.
    let x: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
    let w: [f32; 4] = [0.5, -1.5, 2.0, 0.25];
    let gconst: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    let mut g = Graph::new();
    let nx = g.parameter(t1(&x));
    let nw = g.parameter(t1(&w));
    let ng = g.leaf(t1(&gconst));
    let y = g.rms_norm(nx, nw, 1e-8, &mut budget()).unwrap();
    let p = g.mul(y, ng, &mut budget()).unwrap();
    let l = g.sum(p, &mut budget()).unwrap();
    g.backward(l, None, &mut budget()).unwrap();

    let gx = g.grad(nx).unwrap().to_vec();
    let gw = g.grad(nw).unwrap().to_vec();
    for &v in gx.iter().chain(gw.iter()) {
        assert!(v.is_finite(), "nicht endlich: {v}");
    }
    // dy/dx = w * 1/sqrt(eps) bei x = 0, Verlust ist Summe => Gradient = w*1e4.
    for c in 0..4 {
        let want = w[c] * 1.0e4;
        assert!(
            (gx[c] - want).abs() <= 1e-3 * want.abs(),
            "Index {c}: {} gegen {want}",
            gx[c]
        );
        // dw/dx ist bei x = 0 null, also auch der Parameterbeitrag.
        assert!(gw[c].abs() < 1e-9, "dw[{c}] = {}", gw[c]);
    }
}

#[test]
fn b_11_eps_null_wird_abgelehnt() {
    let x = t1(&[1.0, 2.0]);
    let w = t1(&[1.0, 1.0]);
    let mut b = budget();
    assert_eq!(
        x.rms_norm(&w, 0.0, &mut b).unwrap_err(),
        TensorError::InvalidArgument {
            what: "rms_norm: eps"
        }
    );
    assert_eq!(b.used(), 0, "Ablehnung darf nichts verbuchen");
}

// ------------------------------------------------------------ B-12

#[test]
fn b_12_zwei_rueckwaertspaesse_summe_statt_ueberschreiben() {
    // Teil 1: ein Pfad, zwei Pässe, Erwartung von Hand.
    // L = sum(p ∘ w) => dL/dp = w, nach zwei Pässen 2·w.
    let p: [f32; 2] = [0.8, -1.3];
    let w: [f32; 2] = [0.5, 2.0];

    let mut g = Graph::new();
    let np = g.parameter(t1(&p));
    let nw = g.leaf(t1(&w));
    let m = g.mul(np, nw, &mut budget()).unwrap();
    let l = g.sum(m, &mut budget()).unwrap();

    g.backward(l, None, &mut budget()).unwrap();
    let once = g.grad(np).unwrap().to_vec();
    g.backward(l, None, &mut budget()).unwrap();
    let twice = g.grad(np).unwrap().to_vec();

    for i in 0..2 {
        assert_eq!(once[i], w[i], "erster Pass: {} gegen {}", once[i], w[i]);
        assert_eq!(
            twice[i],
            2.0 * w[i],
            "zweiter Pass: {} gegen {}",
            twice[i],
            2.0 * w[i]
        );
    }

    // Nach zero_grads beginnt die Akkumulation von vorn.
    let mut b = budget();
    g.zero_grads(&mut b);
    assert!(g.grad(np).is_none());
    g.backward(l, None, &mut budget()).unwrap();
    let again = g.grad(np).unwrap().to_vec();
    for i in 0..2 {
        assert_eq!(
            again[i], w[i],
            "nach zero_grads: {} gegen {}",
            again[i], w[i]
        );
    }

    // Teil 2: Raute **und** zwei Pässe in einem Test. Damit deckt derselbe Test
    // beide Akkumulationsstellen ab: die Verteilung innerhalb eines Passes
    // (zweiter Beitrag auf denselben Parameter) und die Addition über Pässe.
    //
    // Erwartung unabhängig von der Implementierung, von Hand:
    //   u = p·W1, v = p·W2, L = sum(u + v)
    //   dL/dp_i = Σ_j (W1[i,j] + W2[i,j])  = Zeilensummen von W1 plus W2
    // W1 = [[1,2],[3,4]] => [3,7];  W2 = [[0.5,-0.5],[2,1]] => [0,3]
    // ein Pass: [3,10];  zwei Pässe: [6,20]. Alles dyadisch, also exakt in f32.
    let w1: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
    let w2: [f32; 4] = [0.5, -0.5, 2.0, 1.0];

    let mut g = Graph::new();
    let np = g.parameter(t2([1, 2], &p));
    let n1 = g.leaf(t2([2, 2], &w1));
    let n2 = g.leaf(t2([2, 2], &w2));
    let u = g.matmul(np, n1, &mut budget()).unwrap();
    let v = g.matmul(np, n2, &mut budget()).unwrap();
    let sum = g.add(u, v, &mut budget()).unwrap();
    let l2 = g.sum(sum, &mut budget()).unwrap();

    g.backward(l2, None, &mut budget()).unwrap();
    let nach_einem = g.grad(np).unwrap().to_vec();
    assert_eq!(nach_einem, vec![3.0, 10.0], "ein Pass, Raute");

    g.backward(l2, None, &mut budget()).unwrap();
    let nach_zwei = g.grad(np).unwrap().to_vec();
    assert_eq!(nach_zwei, vec![6.0, 20.0], "zwei Paesse, Raute");

    // Dritter Durchlauf: die Addition ist fortlaufend, nicht abgeschnitten.
    g.backward(l2, None, &mut budget()).unwrap();
    assert_eq!(
        g.grad(np).unwrap().as_slice(),
        &[9.0, 30.0],
        "drei Paesse, Raute"
    );
}

#[test]
fn b_12_raute_zwei_pfade_auf_dasselbe_blatt() {
    // p wird von zwei Matrixprodukten gelesen. Beide Beiträge müssen addiert
    // werden; ein Überschreiben würde genau einen Pfad verlieren.
    let p: [f32; 2] = [0.8, -1.3];
    let w1: [f32; 4] = [0.5, 2.0, -1.5, 0.25];
    let w2: [f32; 4] = [1.25, -0.5, 0.75, 3.0];

    let mut g = Graph::new();
    // matmul verlangt Rang 2, also p als (1,2); die Leser sind zwei Zeilen.
    let np = g.parameter(t2([1, 2], &p));
    let n1 = g.leaf(t2([2, 2], &w1));
    let n2 = g.leaf(t2([2, 2], &w2));
    let m1 = g.matmul(np, n1, &mut budget()).unwrap();
    let m2 = g.matmul(np, n2, &mut budget()).unwrap();
    let s = g.add(m1, m2, &mut budget()).unwrap();
    let l = g.sum(s, &mut budget()).unwrap();
    g.backward(l, None, &mut budget()).unwrap();

    let grad = g.grad(np).unwrap().to_vec();
    let pf: Vec<f64> = p.iter().map(|&v| v as f64).collect();
    let f1: Vec<f64> = w1.iter().map(|&v| v as f64).collect();
    let f2: Vec<f64> = w2.iter().map(|&v| v as f64).collect();
    let want = fd(
        |v: &[f64]| -> f64 {
            let a = f64_matmul(v, 1, 2, &f1, 2);
            let b = f64_matmul(v, 1, 2, &f2, 2);
            a[0] + a[1] + b[0] + b[1]
        },
        &pf,
        1e-5,
    );
    let e = max_rel(&grad, &want);
    assert!(e < 1e-6, "Rauten-Gradient {e:.3e}");
}

// ------------------------------------------------------------ B-13

#[test]
fn b_13_zusammengesetztes_netz_zwei_schichten() {
    // Kanonische Form ohne Bias-Terme (ATC-AI-LLM-001-A Abschnitt 5.1):
    // x (2,4) -> RMSNorm -> matmul (4,6) -> SwiGLU (6 -> 3) -> matmul (3,4)
    //         -> Kreuzentropie. Alle Parameter liegen im Graphen.
    let x: [f32; 8] = [0.5, -1.25, 0.75, 2.0, -0.3, 0.9, -1.1, 0.4];
    let w0: [f32; 4] = [1.0, 0.5, -0.75, 1.25];
    let w1: [f32; 24] = [
        0.25, -0.5, 0.75, 1.0, -1.25, 0.5, 0.9, 0.1, -0.4, 0.6, 1.1, -0.2, -0.75, 0.35, 0.55, -0.9,
        1.05, 0.3, -0.6, 0.8, 0.2, -1.1, 0.45, 0.65,
    ];
    let w2: [f32; 12] = [
        0.5, -0.25, 0.75, 1.0, -0.5, 0.25, 0.9, -1.1, 0.6, 0.15, -0.35, 0.8,
    ];
    let targets: [u32; 2] = [1, 3];

    let mut g = Graph::new();
    let nx = g.leaf(t2([2, 4], &x));
    let n0 = g.parameter(t1(&w0));
    let n1 = g.parameter(Tensor::new(vec![4, 6], w1.to_vec()).unwrap());
    let n2 = g.parameter(Tensor::new(vec![3, 4], w2.to_vec()).unwrap());

    let y = g.rms_norm(nx, n0, 1e-5, &mut budget()).unwrap();
    let h = g.matmul(y, n1, &mut budget()).unwrap();
    let a = g.swiglu(h, &mut budget()).unwrap();
    let o = g.matmul(a, n2, &mut budget()).unwrap();
    let loss = g.cross_entropy(o, &targets, &mut budget()).unwrap();

    let mut b = budget();
    g.backward(loss, None, &mut b).unwrap();

    let xf: Vec<f64> = x.iter().map(|&v| v as f64).collect();
    let w0f: Vec<f64> = w0.iter().map(|&v| v as f64).collect();
    let w1f: Vec<f64> = w1.iter().map(|&v| v as f64).collect();
    let w2f: Vec<f64> = w2.iter().map(|&v| v as f64).collect();
    let ti: Vec<usize> = targets.iter().map(|&t| t as usize).collect();

    // Vollständige f64-Referenz des Netzes.
    let forward = |w0: &[f64], w1: &[f64], w2: &[f64]| -> f64 {
        let mut logits = vec![0.0f64; 2 * 4];
        for r in 0..2 {
            let y = f64_rms_norm_row(&xf[r * 4..(r + 1) * 4], w0, 1e-5);
            let h = f64_matmul(&y, 1, 4, w1, 6);
            // SwiGLU: silu(a) * b mit a = erste Hälfte, b = zweite Hälfte.
            let mut act = vec![0.0f64; 3];
            for k in 0..3 {
                act[k] = f64_silu(h[k]) * h[3 + k];
            }
            let o = f64_matmul(&act, 1, 3, w2, 4);
            logits[r * 4..(r + 1) * 4].copy_from_slice(&o);
        }
        f64_cross_entropy(&logits, 2, 4, &ti)
    };

    let want0 = fd(|v: &[f64]| forward(v, &w1f, &w2f), &w0f, 1e-5);
    let want1 = fd(|v: &[f64]| forward(&w0f, v, &w2f), &w1f, 1e-5);
    let want2 = fd(|v: &[f64]| forward(&w0f, &w1f, v), &w2f, 1e-5);

    let g0 = g.grad(n0).expect("RMSNorm-Gewicht");
    let g1 = g.grad(n1).expect("W1");
    let g2 = g.grad(n2).expect("W2");
    assert!(
        g.grad(nx).is_none(),
        "Eingabe darf keinen Gradienten halten"
    );

    let e0 = max_rel(g0.as_slice(), &want0);
    let e1 = max_rel(g1.as_slice(), &want1);
    let e2 = max_rel(g2.as_slice(), &want2);
    assert!(e0 < 1e-3, "w0 relative Abweichung {e0:.3e}");
    assert!(e1 < 1e-3, "W1 relative Abweichung {e1:.3e}");
    assert!(e2 < 1e-3, "W2 relative Abweichung {e2:.3e}");

    // Der Verlust muss endlich und positiv sein.
    let lv = g.value(loss).unwrap().as_slice()[0];
    assert!(lv.is_finite() && lv > 0.0, "Verlust {lv}");
}

// ------------------------------------------------ Budget und Determinismus

#[test]
fn rueckwaertspass_ist_transaktional() {
    // Budget 0: der Rückwärtspass darf nicht teilweise verbuchen.
    let x = [0.4f32, -1.2];
    let w = [0.5f32, 2.0];
    let mut g = Graph::new();
    let nx = g.parameter(t1(&x));
    let nw = g.leaf(t1(&w));
    let m = {
        let mut fb = budget();
        g.mul(nx, nw, &mut fb).unwrap()
    };
    let l = {
        let mut fb = budget();
        g.sum(m, &mut fb).unwrap()
    };

    let mut empty = Budget::new(0);
    let err = g.backward(l, None, &mut empty).unwrap_err();
    assert_eq!(
        err,
        TensorError::BudgetExceeded {
            requested: 4,
            limit: 0
        }
    );
    assert_eq!(empty.used(), 0, "Fehler hinterlaesst eine Buchung");
    assert!(g.grad(nx).is_none(), "Fehler hinterlaesst einen Gradienten");
}

#[test]
fn graph_gibt_alles_zurueck() {
    let mut b = budget();
    let base = b.used();
    let mut g = Graph::new();
    let np = g.parameter(t1(&[1.0, 2.0]));
    let nw = g.leaf(t1(&[0.5, -1.5]));
    let m = g.mul(np, nw, &mut b).unwrap();
    let l = g.sum(m, &mut b).unwrap();
    g.backward(l, None, &mut b).unwrap();
    assert!(b.used() > base, "Graph muss Bytes halten");
    assert!(g.held_bytes() > 0);
    g.clear(&mut b);
    assert_eq!(b.used(), base, "clear gibt nicht alles zurueck");
    assert_eq!(g.held_bytes(), 0);
    assert!(g.is_empty());
}

#[test]
fn rueckwaertspass_ist_deterministisch() {
    let x = [0.37f32, -1.25, 2.1, 0.9];
    let w = [1.7f32, -0.6, 0.45, 2.3];
    let run = || -> Vec<f32> {
        let mut g = Graph::new();
        let nx = g.parameter(t2([2, 2], &x));
        let nw = g.leaf(t2([2, 2], &w));
        let m = g.matmul(nx, nw, &mut budget()).unwrap();
        let t = g.transpose(m, &mut budget()).unwrap();
        let s = g.softmax(t, &mut budget()).unwrap();
        let l = g.sum(s, &mut budget()).unwrap();
        g.backward(l, None, &mut budget()).unwrap();
        g.grad(nx).unwrap().to_vec()
    };
    assert_eq!(run(), run(), "zwei Laeufe muessen bitgleich sein");
}

#[test]
fn unbekannter_knoten_ist_typisiert() {
    let mut g = Graph::new();
    assert_eq!(g.value(7).unwrap_err(), TensorError::UnknownNode);
    assert_eq!(
        g.backward(7, None, &mut budget()).unwrap_err(),
        TensorError::UnknownNode
    );
}

#[test]
fn nicht_skalarer_ausgang_ohne_seed_wird_abgelehnt() {
    let mut g = Graph::new();
    let nx = g.parameter(t1(&[1.0, 2.0]));
    assert_eq!(
        g.backward(nx, None, &mut budget()).unwrap_err(),
        TensorError::ShapeMismatch
    );
    // Mit passendem Seed geht es: Gradient ist der Seed selbst.
    let seed = Tensor::from_vec(vec![2.0, -3.0]);
    g.backward(nx, Some(&seed), &mut budget()).unwrap();
    assert_eq!(g.grad(nx).unwrap().as_slice(), &[2.0, -3.0]);
}

// atc-aurora-tensor — Kreuzentropie (M-02, Testfall B-08)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Kreuzentropie als Log-Softmax, nicht als log(softmax(x)). Der Unterschied ist
// kein Stil: bei einer Zielwahrscheinlichkeit von 0 entsteht in der zweiten Form
// log(0) und damit -inf. Der Contract verlangt für diesen Fall einen endlichen
// Wert (B-08).
//
// Der Gradient vereinfacht sich bei dieser Form zu `softmax(x) - target`. Er wird
// hier mitgeliefert, weil er ohne Zusatzaufwand aus demselben Log-Softmax fällt
// und weil B-08 ihn exakt so prüft.

use alloc::vec;

use crate::budget::Budget;
use crate::error::{Result, TensorError};
use crate::tensor::Tensor;

/// Kreuzentropie über die letzte Achse, Summe über die Zeilen.
///
/// Rückgabe: `(Verlust als Skalar, Gradient bezüglich der Logits)`. Der
/// Gradient ist exakt `softmax(logits) - onehot(target)`, Zeile für Zeile.
///
/// Die Reduktion ist die Summe und nicht der Mittelwert, weil der Testfall B-08
/// den Gradienten ausdrücklich als `softmax - target` festlegt; bei einem
/// Mittelwert käme ein Faktor `1/n` hinzu und die Bedingung wäre nur für eine
/// Zeile erfüllt. Die Mittelung über einen Stapel gehört in die Trainingsschleife
/// (M-06) und nicht in die Verlustfunktion, sonst gäbe es zwei Definitionen.
pub fn cross_entropy(
    logits: &Tensor,
    targets: &[u32],
    budget: &mut Budget,
) -> Result<(Tensor, Tensor)> {
    let rank = logits.rank();
    if rank != 2 {
        return Err(TensorError::UnsupportedRank {
            got: rank,
            expected: "rank 2",
        });
    }
    let (n, v) = (logits.shape()[0], logits.shape()[1]);
    if targets.len() != n {
        return Err(TensorError::ShapeMismatch);
    }
    if v == 0 || n == 0 {
        return Err(TensorError::ShapeMismatch);
    }
    for &t in targets {
        if t as usize >= v {
            return Err(TensorError::ShapeMismatch);
        }
    }

    // Beide Puffer werden vor der Zuteilung geprüft: erst Verlust (1 Element),
    // dann der Gradient (n*v). Schlägt der zweite fehl, wird der erste
    // freigegeben, damit kein halber Zustand im Budget bleibt.
    let mut loss_buf = Tensor::zeros(vec![], budget)?;
    let mut grad = match Tensor::zeros(logits.shape().to_vec(), budget) {
        Ok(g) => g,
        Err(e) => {
            budget.release(crate::tensor::BYTES_PER_ELEMENT);
            return Err(e);
        }
    };

    let src = logits.as_slice();
    let g = grad.as_mut_slice();
    let mut loss_sum: f32 = 0.0;

    for r in 0..n {
        let row = &src[r * v..(r + 1) * v];
        let mut mx = row[0];
        for &x in row {
            if x > mx {
                mx = x;
            }
        }
        let mut sum_exp: f32 = 0.0;
        for &x in row {
            sum_exp += libm::expf(x - mx);
        }
        // log-sum-exp in der stabilen Form; bei sum_exp == 1 ist der Term
        // einfach mx, und der Verlust bleibt endlich, auch wenn die
        // Zielwahrscheinlichkeit unter f32 auf 0 rundet.
        let log_sum_exp = mx + libm::logf(sum_exp);
        let t = targets[r] as usize;
        loss_sum += log_sum_exp - row[t];
        for c in 0..v {
            let p = libm::expf(row[c] - mx) / sum_exp;
            let y = if c == t { 1.0 } else { 0.0 };
            g[r * v + c] = p - y;
        }
    }

    loss_buf.as_mut_slice()[0] = loss_sum;
    Ok((loss_buf, grad))
}

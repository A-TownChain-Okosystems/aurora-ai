// atc-aurora-tensor — Normierung und Aktivierung (M-02)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Zwei Operationen, die der kanonischen Ref-1-Konfiguration entsprechen
// (ATC-AI-LLM-001-A Abschnitt 5.1): RMSNorm mit Pre-Norm und SwiGLU ohne
// Bias-Terme. Beide sind zeilenweise definiert, weil die Bezugsachse die letzte
// Dimension ist.
//
// Die Ableitungen liegen in `autograd.rs`; hier steht nur der Vorwärtspfad. Das
// ist Absicht: es gibt genau eine Stelle je Operation, an der die Ableitung
// steht, und keine zweite semantische Kopie daneben.

use alloc::vec;

use crate::budget::Budget;
use crate::error::{Result, TensorError};
use crate::tensor::{alloc_zeroed, Tensor};

impl Tensor {
    /// RMS-Norm über die letzte Achse: `y = x / sqrt(mean(x^2) + eps) * w`.
    ///
    /// `w` ist ein Rang-1-Tensor mit der Breite der letzten Achse. `eps` muss
    /// positiv und endlich sein; `eps <= 0` wäre eine Division durch Null und
    /// wird deshalb typisiert abgelehnt, nicht als `NaN` durchgereicht.
    ///
    /// Die Wurzel kommt aus `libm::sqrtf`, damit das Crate ohne std baut.
    pub fn rms_norm(&self, w: &Self, eps: f32, budget: &mut Budget) -> Result<Tensor> {
        let rank = self.rank();
        if rank != 1 && rank != 2 {
            return Err(TensorError::UnsupportedRank {
                got: rank,
                expected: "rank 1 or 2",
            });
        }
        if w.rank() != 1 {
            return Err(TensorError::UnsupportedRank {
                got: w.rank(),
                expected: "rank 1",
            });
        }
        if !eps.is_finite() || eps <= 0.0 {
            return Err(TensorError::InvalidArgument {
                what: "rms_norm: eps",
            });
        }
        let d = self.shape()[rank - 1];
        if w.shape()[0] != d {
            return Err(TensorError::ShapeMismatch);
        }
        if d == 0 {
            return Err(TensorError::ShapeMismatch);
        }

        let mut data = alloc_zeroed(self.size(), budget)?;
        let src = self.as_slice();
        let wv = w.as_slice();
        for (row_out, row_in) in data.chunks_mut(d).zip(src.chunks(d)) {
            let mut acc: f32 = 0.0;
            for &v in row_in {
                acc += v * v;
            }
            let mean = acc / d as f32;
            let inv = 1.0 / libm::sqrtf(mean + eps);
            for ((o, &v), &wj) in row_out.iter_mut().zip(row_in).zip(wv) {
                *o = v * inv * wj;
            }
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// SwiGLU über die letzte Achse: `y = silu(a) * b` mit `a = x[..h]`,
    /// `b = x[h..]` und `silu(t) = t * sigmoid(t)`.
    ///
    /// Die letzte Achse muss gerade sein; `h = Breite / 2`. Der Contract nennt
    /// nur den Namen `swiglu`, nicht die Aufteilung. Diese Aufteilung ist die in
    /// `AUTOGRAD-CONTRACT.md` festgelegte und getestete Variante.
    pub fn swiglu(&self, budget: &mut Budget) -> Result<Tensor> {
        let rank = self.rank();
        if rank != 2 {
            return Err(TensorError::UnsupportedRank {
                got: rank,
                expected: "rank 2",
            });
        }
        let (rows, cols) = (self.shape()[0], self.shape()[1]);
        if cols == 0 || cols % 2 != 0 {
            return Err(TensorError::ShapeMismatch);
        }
        let h = cols / 2;
        let out_len = rows.checked_mul(h).ok_or(TensorError::SizeOverflow)?;
        let mut data = alloc_zeroed(out_len, budget)?;
        let src = self.as_slice();
        for r in 0..rows {
            for k in 0..h {
                let a = src[r * cols + k];
                let b = src[r * cols + h + k];
                data[r * h + k] = silu(a) * b;
            }
        }
        Tensor::new(vec![rows, h], data)
    }
}

/// SiLU, `t * sigmoid(t)`.
pub(crate) fn silu(t: f32) -> f32 {
    t / (1.0 + libm::expf(-t))
}

/// Ableitung von SiLU: `sigmoid(t) * (1 + t * (1 - sigmoid(t)))`.
///
/// Steht hier und nicht in `autograd.rs`, weil sie zur Definition der
/// Aktivierung gehört: wäre sie doppelt notiert, könnten Vorwärts- und
/// Rückwärtspfad auseinanderlaufen, ohne dass ein Test das merkt.
pub(crate) fn silu_grad(t: f32) -> f32 {
    let s = 1.0 / (1.0 + libm::expf(-t));
    s * (1.0 + t * (1.0 - s))
}

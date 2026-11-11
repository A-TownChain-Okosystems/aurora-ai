// atc-aurora-tensor — Operationen (M-02)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Elementweise Operationen verlangen gleiche Formen. Es gibt kein implizites
// Broadcasting: eine Form wie (3,1)+(1,4) ist ein Fehler, kein Sonderfall
// (Testfall B-02). Jede zuteilende Operation nimmt ein Budget entgegen und
// prüft es vor der Zuteilung.

use crate::budget::Budget;
use crate::error::{Result, TensorError};
use crate::tensor::{alloc_zeroed, Tensor};

impl Tensor {
    /// Elementweise Addition. Gleiche Formen erforderlich.
    pub fn add(&self, other: &Self, budget: &mut Budget) -> Result<Tensor> {
        if !self.same_shape(other) {
            return Err(TensorError::ShapeMismatch);
        }
        let mut data = alloc_zeroed(self.size(), budget)?;
        for (d, (a, b)) in data
            .iter_mut()
            .zip(self.as_slice().iter().zip(other.as_slice()))
        {
            *d = a + b;
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// Elementweise Multiplikation. Gleiche Formen erforderlich.
    pub fn mul(&self, other: &Self, budget: &mut Budget) -> Result<Tensor> {
        if !self.same_shape(other) {
            return Err(TensorError::ShapeMismatch);
        }
        let mut data = alloc_zeroed(self.size(), budget)?;
        for (d, (a, b)) in data
            .iter_mut()
            .zip(self.as_slice().iter().zip(other.as_slice()))
        {
            *d = a * b;
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// Multiplikation mit einem Skalar.
    pub fn scale(&self, factor: f32, budget: &mut Budget) -> Result<Tensor> {
        let mut data = alloc_zeroed(self.size(), budget)?;
        for (d, s) in data.iter_mut().zip(self.as_slice()) {
            *d = s * factor;
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// Matrixprodukt zweier Tensoren vom Rang 2 (B-01).
    ///
    /// `(m,k) x (k,n) -> (m,n)`. Unpassende Innenmaße sind ein `ShapeMismatch`,
    /// kein Panik und kein stilles Auffüllen.
    pub fn matmul(&self, other: &Self, budget: &mut Budget) -> Result<Tensor> {
        if self.rank() != 2 {
            return Err(TensorError::UnsupportedRank {
                got: self.rank(),
                expected: "rank 2",
            });
        }
        if other.rank() != 2 {
            return Err(TensorError::UnsupportedRank {
                got: other.rank(),
                expected: "rank 2",
            });
        }
        let (m, k) = (self.shape()[0], self.shape()[1]);
        let (k2, n) = (other.shape()[0], other.shape()[1]);
        if k != k2 {
            return Err(TensorError::ShapeMismatch);
        }
        let out_len = m.checked_mul(n).ok_or(TensorError::SizeOverflow)?;
        let mut data = alloc_zeroed(out_len, budget)?;
        let a = self.as_slice();
        let b = other.as_slice();
        for i in 0..m {
            for j in 0..n {
                let mut acc: f32 = 0.0;
                for x in 0..k {
                    acc += a[i * k + x] * b[x * n + j];
                }
                data[i * n + j] = acc;
            }
        }
        Tensor::new(alloc::vec![m, n], data)
    }

    /// Transponiert einen Tensor vom Rang 2.
    pub fn transpose(&self, budget: &mut Budget) -> Result<Tensor> {
        if self.rank() != 2 {
            return Err(TensorError::UnsupportedRank {
                got: self.rank(),
                expected: "rank 2",
            });
        }
        let (r, c) = (self.shape()[0], self.shape()[1]);
        let mut data = alloc_zeroed(self.size(), budget)?;
        let src = self.as_slice();
        for i in 0..r {
            for j in 0..c {
                data[j * r + i] = src[i * c + j];
            }
        }
        Tensor::new(alloc::vec![c, r], data)
    }

    /// Softmax zeilenweise, numerisch stabil durch Subtraktion des Maximums.
    pub fn softmax(&self, budget: &mut Budget) -> Result<Tensor> {
        let rank = self.rank();
        if rank != 1 && rank != 2 {
            return Err(TensorError::UnsupportedRank {
                got: rank,
                expected: "rank 1 or 2",
            });
        }
        let mut data = alloc_zeroed(self.size(), budget)?;
        let src = self.as_slice();
        let cols = if rank == 1 {
            src.len()
        } else {
            self.shape()[1]
        };
        if src.is_empty() || cols == 0 {
            return Tensor::new(self.shape().to_vec(), data);
        }
        for (row_out, row_in) in data.chunks_mut(cols).zip(src.chunks(cols)) {
            let mut mx = row_in[0];
            for &v in row_in {
                if v > mx {
                    mx = v;
                }
            }
            let mut sum: f32 = 0.0;
            for (o, &v) in row_out.iter_mut().zip(row_in) {
                let e = libm::expf(v - mx);
                *o = e;
                sum += e;
            }
            if sum > 0.0 {
                for o in row_out.iter_mut() {
                    *o /= sum;
                }
            } else {
                // Alle Eingaben waren -inf oder der Ausdruck lief ins Leere.
                // Gleichverteilung ist die einzige Antwort, die nicht NaN ist.
                let uniform = 1.0 / cols as f32;
                for o in row_out.iter_mut() {
                    *o = uniform;
                }
            }
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// ReLU, elementweise.
    pub fn relu(&self, budget: &mut Budget) -> Result<Tensor> {
        let mut data = alloc_zeroed(self.size(), budget)?;
        for (d, &v) in data.iter_mut().zip(self.as_slice()) {
            *d = if v > 0.0 { v } else { 0.0 };
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// Sigmoid, elementweise, über `libm::expf`.
    pub fn sigmoid(&self, budget: &mut Budget) -> Result<Tensor> {
        let mut data = alloc_zeroed(self.size(), budget)?;
        for (d, &v) in data.iter_mut().zip(self.as_slice()) {
            *d = 1.0 / (1.0 + libm::expf(-v));
        }
        Tensor::new(self.shape().to_vec(), data)
    }

    /// Skalarprodukt zweier formgleicher Tensoren, über alle Elemente.
    pub fn dot(&self, other: &Self) -> Result<f32> {
        if !self.same_shape(other) {
            return Err(TensorError::ShapeMismatch);
        }
        let mut acc: f32 = 0.0;
        for (a, b) in self.as_slice().iter().zip(other.as_slice()) {
            acc += a * b;
        }
        Ok(acc)
    }

    /// Summe aller Elemente.
    pub fn sum(&self) -> f32 {
        let mut acc: f32 = 0.0;
        for &v in self.as_slice() {
            acc += v;
        }
        acc
    }

    /// Mittelwert aller Elemente. Leerer Tensor ergibt 0.0.
    pub fn mean(&self) -> f32 {
        if self.size() == 0 {
            return 0.0;
        }
        self.sum() / self.size() as f32
    }
}

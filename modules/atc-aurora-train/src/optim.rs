// atc-aurora-train — Optimierer (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Ein Optimierer, kein Baukasten: Adam mit entkoppelter Gewichtsdaempfung
// (AdamW). SGD mit Momentum ist als zweite, kleinere Variante enthalten, weil
// ein Referenzlauf ohne Vergleichsbasis schlecht einzuordnen ist; beide benutzen
// dieselbe Schnittstelle und dieselben Gradienten.

use atc_aurora_model::weights::Weights;
use atc_aurora_tensor::Tensor;

use crate::error::TrainError;
use crate::gradients::Gradients;

/// Hyperparameter eines Laufs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HyperParams {
    /// Lernrate.
    pub lr: f32,
    /// Zerfall des ersten Moments (Adam).
    pub beta1: f32,
    /// Zerfall des zweiten Moments (Adam).
    pub beta2: f32,
    /// Stabilisierung der Division.
    pub eps: f32,
    /// Entkoppelte Gewichtsdaempfung (AdamW).
    pub weight_decay: f32,
    /// Momentum fuer SGD; bei Adam ohne Wirkung.
    pub momentum: f32,
    /// Obergrenze der Gradientennorm; `None` schaltet die Begrenzung ab.
    pub clip_norm: Option<f32>,
    /// Bytebudget je Sequenz im Vorwaerts- und Rueckwaertspass.
    pub budget_bytes: usize,
}

impl HyperParams {
    /// Voreinstellung fuer Adam: beta1 0,9 · beta2 0,999 · eps 1e-8,
    /// keine Gewichtsdaempfung, keine Begrenzung, 256 MiB Budget.
    pub const fn adam(lr: f32) -> Self {
        HyperParams {
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
            weight_decay: 0.0,
            momentum: 0.0,
            clip_norm: None,
            budget_bytes: 256 << 20,
        }
    }

    /// Prueft die Bereiche. Eine Lernrate von 0 oder NaN ist kein Sonderfall,
    /// sondern ein Aufruf, der nichts bewirken kann, und wird abgelehnt.
    pub fn validate(&self) -> Result<(), TrainError> {
        let finite_positive = |v: f32| v.is_finite() && v > 0.0;
        if !finite_positive(self.lr) {
            return Err(TrainError::InvalidHyperparameter { what: "lr" });
        }
        if !(0.0..1.0).contains(&self.beta1) {
            return Err(TrainError::InvalidHyperparameter { what: "beta1" });
        }
        if !(0.0..1.0).contains(&self.beta2) {
            return Err(TrainError::InvalidHyperparameter { what: "beta2" });
        }
        if !finite_positive(self.eps) {
            return Err(TrainError::InvalidHyperparameter { what: "eps" });
        }
        if !self.weight_decay.is_finite() || self.weight_decay < 0.0 {
            return Err(TrainError::InvalidHyperparameter {
                what: "weight_decay",
            });
        }
        if !(0.0..1.0).contains(&self.momentum) {
            return Err(TrainError::InvalidHyperparameter { what: "momentum" });
        }
        if let Some(c) = self.clip_norm {
            if !finite_positive(c) {
                return Err(TrainError::InvalidHyperparameter { what: "clip_norm" });
            }
        }
        if self.budget_bytes == 0 {
            return Err(TrainError::InvalidHyperparameter {
                what: "budget_bytes",
            });
        }
        Ok(())
    }
}

/// Adam mit entkoppelter Gewichtsdaempfung.
#[derive(Debug, Clone)]
pub struct Adam {
    hp: HyperParams,
    /// Anzahl der Schritte; traegt die Bias-Korrektur und den Pruefpunkt.
    t: u64,
    m: Vec<Vec<f32>>,
    v: Vec<Vec<f32>>,
}

impl Adam {
    /// Neuer Optimierer mit Nullmomenten in der Form der Gewichte.
    pub fn new(hp: HyperParams, w: &Weights) -> Result<Self, TrainError> {
        hp.validate()?;
        let tensors = w.tensors();
        Ok(Adam {
            hp,
            t: 0,
            m: tensors.iter().map(|p| vec![0.0f32; p.size()]).collect(),
            v: tensors.iter().map(|p| vec![0.0f32; p.size()]).collect(),
        })
    }

    /// Optimierer aus einem gespeicherten Zustand. Die Laengen muessen zu den
    /// Parametern passen, sonst waere der fortgesetzte Lauf ein anderer Lauf.
    pub fn from_state(
        hp: HyperParams,
        w: &Weights,
        t: u64,
        m: Vec<Vec<f32>>,
        v: Vec<Vec<f32>>,
    ) -> Result<Self, TrainError> {
        hp.validate()?;
        let tensors = w.tensors();
        if m.len() != tensors.len() || v.len() != tensors.len() {
            return Err(TrainError::CorruptState {
                what: "moment_count",
            });
        }
        for (i, p) in tensors.iter().enumerate() {
            if m[i].len() != p.size() || v[i].len() != p.size() {
                return Err(TrainError::CorruptState {
                    what: "moment_shape",
                });
            }
        }
        Ok(Adam { hp, t, m, v })
    }

    pub fn hyperparams(&self) -> &HyperParams {
        &self.hp
    }

    pub fn step_count(&self) -> u64 {
        self.t
    }

    pub fn first_moment(&self) -> &[Vec<f32>] {
        &self.m
    }

    pub fn second_moment(&self) -> &[Vec<f32>] {
        &self.v
    }

    /// Ein Optimierungsschritt. Erhoeht `t` genau einmal, auch wenn der
    /// Schritt in der Mitte scheitert? Nein: geprueft wird vorher, damit ein
    /// abgelehnter Schritt den Zustand nicht verschiebt.
    pub fn step(
        &mut self,
        params: &mut [&mut Tensor],
        grads: &Gradients,
    ) -> Result<(), TrainError> {
        if params.len() != self.m.len() || grads.len() != params.len() {
            return Err(TrainError::WeightMismatch {
                what: "optimizer_state",
            });
        }
        for (i, p) in params.iter().enumerate() {
            if p.size() != grads.get(i).len() {
                return Err(TrainError::WeightMismatch {
                    what: "gradient_shape",
                });
            }
        }

        self.t += 1;
        let (lr, b1, b2, eps, wd) = (
            self.hp.lr,
            self.hp.beta1,
            self.hp.beta2,
            self.hp.eps,
            self.hp.weight_decay,
        );
        let t = self.t as i32;
        let bc1 = 1.0 - b1.powi(t);
        let bc2 = 1.0 - b2.powi(t);

        for (i, p) in params.iter_mut().enumerate() {
            let g = grads.get(i);
            let m = &mut self.m[i];
            let v = &mut self.v[i];
            let mut idx = 0usize;
            p.map_in_place(|w| {
                let gi = g[idx];
                m[idx] = b1 * m[idx] + (1.0 - b1) * gi;
                v[idx] = b2 * v[idx] + (1.0 - b2) * gi * gi;
                let mh = m[idx] / bc1;
                let vh = v[idx] / bc2;
                let update = mh / (vh.sqrt() + eps);
                idx += 1;
                w - lr * update - lr * wd * w
            });
        }
        Ok(())
    }
}

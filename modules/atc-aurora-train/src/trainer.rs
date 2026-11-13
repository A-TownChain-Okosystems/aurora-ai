// atc-aurora-train — Trainingsschleife (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Ein Schritt besteht aus: Graph je Sequenz bauen, Kreuzentropie, Rueckwaerts-
// pass, Beitraege einsammeln, durch die Zahl der Zielpositionen teilen,
// optional begrenzen, Optimierer anwenden, Zustand aufraeumen. Die Reihenfolge
// ist bindend, weil der Verlust sonst von der Stapelgroesse abhinge.
//
// Determinismus: es gibt keinen Zufallsstrom. Ein Stapel, eine Epoche und ein
// Startgewicht ergeben denselben Verlauf, auf jeder Maschine.

use atc_aurora_model::config::Config;
use atc_aurora_model::forward::ForwardError;
use atc_aurora_model::graph::forward_graph;
use atc_aurora_model::weights::Weights;
use atc_aurora_tensor::Budget;

use crate::error::TrainError;
use crate::gradients::Gradients;
use crate::optim::{Adam, HyperParams};
use crate::registry::{checkpoint_from, Checkpoint};
use crate::state::TrainerState;

/// Ergebnis eines Schritts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepReport {
    /// Nummer des Schritts, beginnend bei 1.
    pub step: u64,
    /// Mittlerer Verlust je Zielposition in diesem Schritt.
    pub loss: f32,
    /// Norm des Gradienten vor einer Begrenzung.
    pub grad_norm: f32,
    /// Zahl der Zielpositionen im Stapel.
    pub tokens: usize,
    /// Angewendete Begrenzung, falls eine konfiguriert war.
    pub clip_norm: Option<f32>,
}

/// Trainingsschleife mit Gewichten, Optimierer und Buchhaltung.
pub struct Trainer {
    cfg: Config,
    weights: Weights,
    adam: Adam,
    grads: Gradients,
    step: u64,
    tokens_seen: u64,
    history: Vec<f32>,
}

impl Trainer {
    /// Neuer Lauf. Die Gewichte werden uebernommen, nicht kopiert erzeugt:
    /// der Aufrufer entscheidet ueber die Initialisierung.
    pub fn new(cfg: Config, weights: Weights, hp: HyperParams) -> Result<Self, TrainError> {
        cfg.validate()
            .map_err(|_| TrainError::WeightMismatch { what: "config" })?;
        if weights.blocks.len() != cfg.n_layers {
            return Err(TrainError::WeightMismatch { what: "blocks" });
        }
        let adam = Adam::new(hp, &weights)?;
        let grads = Gradients::zeros_like(&weights);
        Ok(Trainer {
            cfg,
            weights,
            adam,
            grads,
            step: 0,
            tokens_seen: 0,
            history: Vec::new(),
        })
    }

    /// Lauf aus einem Pruefpunkt. Gewichte, Schritt, Hyperparameter und
    /// Optimierermomente kommen aus dem Pruefpunkt; der Verlauf beginnt leer,
    /// weil er nicht Teil des Zustands ist.
    pub fn from_checkpoint(
        cfg: Config,
        checkpoint: &Checkpoint,
        max_bytes: usize,
    ) -> Result<Self, TrainError> {
        let (weights, state) = crate::registry::restore(&cfg, checkpoint, max_bytes)?;
        let adam = Adam::from_state(state.hp, &weights, state.adam_t, state.m, state.v)?;
        let grads = Gradients::zeros_like(&weights);
        Ok(Trainer {
            cfg,
            weights,
            adam,
            grads,
            step: state.step,
            tokens_seen: state.tokens_seen,
            history: Vec::new(),
        })
    }

    pub fn weights(&self) -> &Weights {
        &self.weights
    }

    pub fn hyperparams(&self) -> &HyperParams {
        self.adam.hyperparams()
    }

    pub fn step_count(&self) -> u64 {
        self.step
    }

    pub fn tokens_seen(&self) -> u64 {
        self.tokens_seen
    }

    /// Verlustverlauf je Schritt.
    pub fn history(&self) -> &[f32] {
        &self.history
    }

    /// Aktueller Zustand fuer einen Pruefpunkt.
    pub fn state(&self) -> TrainerState {
        TrainerState {
            step: self.step,
            tokens_seen: self.tokens_seen,
            hp: *self.adam.hyperparams(),
            adam_t: self.adam.step_count(),
            m: self.adam.first_moment().to_vec(),
            v: self.adam.second_moment().to_vec(),
        }
    }

    /// Pruefpunkt des aktuellen Zustands (Gewichte als ATCM plus Zustand).
    pub fn checkpoint(&self) -> Result<Checkpoint, TrainError> {
        checkpoint_from(&self.cfg, &self.weights, &self.state())
    }

    /// Ein Optimierungsschritt ueber einen Stapel.
    ///
    /// Alle Sequenzen eines Stapels sind unabhaengig; je Sequenz entsteht ein
    /// eigener Graph, weil die kausale Attention nicht ueber Sequenzgrenzen
    /// hinwegsehen darf. Die Beitraege werden addiert und durch die Zahl der
    /// Zielpositionen geteilt, damit der Verlust nicht von der Stapelgroesse
    /// abhaengt.
    pub fn step(&mut self, batch: &[Vec<u32>]) -> Result<StepReport, TrainError> {
        if batch.is_empty() {
            return Err(TrainError::InvalidBatch);
        }
        let mut total = 0usize;
        for seq in batch {
            if seq.len() < 2 {
                return Err(TrainError::InvalidBatch);
            }
            total += seq.len() - 1;
        }
        if total == 0 {
            return Err(TrainError::InvalidBatch);
        }

        self.grads.zero();
        let mut loss_sum = 0.0f32;
        for seq in batch {
            let input = &seq[..seq.len() - 1];
            let targets = &seq[1..];
            let mut budget = Budget::new(self.adam.hyperparams().budget_bytes);
            let mut g = match forward_graph(&self.cfg, &self.weights, input, &mut budget) {
                Ok(g) => g,
                Err(e) => return Err(TrainError::Forward(e)),
            };
            let loss_node = g.graph.cross_entropy(g.logits, targets, &mut budget)?;
            loss_sum += g.graph.value(loss_node)?.as_slice()[0];
            // Mittelwert ueber den ganzen Stapel: der Faktor geht in den
            // Graphen, nicht in eine Nachbearbeitung der Gradienten.
            let scaled = g.graph.scale(loss_node, 1.0 / total as f32, &mut budget)?;
            g.graph.backward(scaled, None, &mut budget)?;
            self.grads.accumulate(&g.graph, &g.params)?;
            g.graph.clear(&mut budget);
        }

        let norm = self.grads.l2_norm();
        let limit = self.adam.hyperparams().clip_norm;
        if let Some(max) = limit {
            self.grads.clip_by_norm(max);
        }

        // Getrennte Feldleihen, damit Gewichte und Optimierer gleichzeitig
        // benutzt werden koennen.
        let Trainer {
            weights,
            adam,
            grads,
            ..
        } = self;
        let mut params = weights.tensors_mut();
        adam.step(&mut params, grads)?;

        self.step += 1;
        self.tokens_seen += total as u64;
        let loss = loss_sum / total as f32;
        self.history.push(loss);
        Ok(StepReport {
            step: self.step,
            loss,
            grad_norm: norm,
            tokens: total,
            clip_norm: limit,
        })
    }

    /// Feste Zahl von Schritten ueber einen Korpus, Stapel der Reihe nach.
    ///
    /// Kein Mischen: die Reihenfolge ist Teil der Reproduzierbarkeit. Wer
    /// mischen will, uebergibt die Stapel selbst an [`Trainer::step`].
    pub fn fit(
        &mut self,
        corpus: &[Vec<u32>],
        batch_size: usize,
        steps: usize,
    ) -> Result<Vec<f32>, TrainError> {
        if corpus.is_empty() || batch_size == 0 {
            return Err(TrainError::InvalidBatch);
        }
        let mut losses = Vec::with_capacity(steps);
        let mut cursor = 0usize;
        for _ in 0..steps {
            let mut batch = Vec::with_capacity(batch_size);
            for _ in 0..batch_size {
                batch.push(corpus[cursor % corpus.len()].clone());
                cursor += 1;
            }
            losses.push(self.step(&batch)?.loss);
        }
        Ok(losses)
    }
}

/// Wandelt einen Vorwaertsfehler in den Trainingsfehler; fuer Aufrufer, die
/// `?` auf `forward` anwenden wollen.
pub fn forward_error(e: ForwardError) -> TrainError {
    TrainError::Forward(e)
}

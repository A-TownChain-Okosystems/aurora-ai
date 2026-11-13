// atc-aurora-train — Gradientenspeicher (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Der Graph aus M-03 haelt Gradienten je Knoten, aber nur fuer **einen** Aufruf.
// Ein Stapel besteht aus mehreren Sequenzen, und jede Sequenz ist ein eigener
// Graph, weil die Attention innerhalb einer Sequenz kausal ist und nicht ueber
// Stapelgrenzen hinweg sieht. Deshalb sammelt dieser Speicher die Beitraege der
// Sequenzen ein und haelt sie in der kanonischen Parameterfolge
// (`Weights::tensors`): Index im Gradienten ist Index des Parameters.

use atc_aurora_model::weights::Weights;
use atc_aurora_tensor::{Graph, NodeId};

use crate::error::TrainError;

/// Aufsummierte Gradienten je Parameter.
#[derive(Debug, Clone)]
pub struct Gradients {
    buffers: Vec<Vec<f32>>,
}

impl Gradients {
    /// Nullspeicher in der Form der Gewichte.
    pub fn zeros_like(w: &Weights) -> Self {
        Gradients {
            buffers: w.tensors().iter().map(|t| vec![0.0f32; t.size()]).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Gradient eines Parameters.
    pub fn get(&self, index: usize) -> &[f32] {
        &self.buffers[index]
    }

    pub fn buffers(&self) -> &[Vec<f32>] {
        &self.buffers
    }

    /// Setzt alle Beitraege zurueck.
    pub fn zero(&mut self) {
        for b in &mut self.buffers {
            for v in b.iter_mut() {
                *v = 0.0;
            }
        }
    }

    /// Addiert die Parametergradienten eines Graphen.
    ///
    /// Ein Parameter ohne Gradient bleibt unveraendert; das ist kein Fehler,
    /// sondern der Normalfall fuer einen Block, den die Sequenz nicht erreicht
    /// hat. Formabweichungen dagegen werden abgelehnt: sonst wuerde ein falsch
    /// zugeordneter Parameter still den Gradienten eines anderen bekommen.
    pub fn accumulate(&mut self, graph: &Graph, params: &[NodeId]) -> Result<(), TrainError> {
        if params.len() != self.buffers.len() {
            return Err(TrainError::WeightMismatch {
                what: "parameter_count",
            });
        }
        for (i, node) in params.iter().enumerate() {
            let Some(g) = graph.grad(*node) else { continue };
            if g.size() != self.buffers[i].len() {
                return Err(TrainError::WeightMismatch {
                    what: "gradient_shape",
                });
            }
            for (dst, src) in self.buffers[i].iter_mut().zip(g.as_slice()) {
                *dst += *src;
            }
        }
        Ok(())
    }

    /// Setzt den Gradienten eines Parameters.
    ///
    /// Gedacht fuer Aufrufer, die ihren Gradienten selbst bilden (etwa einen
    /// numerischen Abgleich); der Trainingspfad benutzt ausschliesslich
    /// [`Gradients::accumulate`].
    pub fn set(&mut self, index: usize, values: &[f32]) {
        let dst = &mut self.buffers[index];
        for (d, s) in dst.iter_mut().zip(values) {
            *d = *s;
        }
    }

    /// Multipliziert alle Beitraege mit einem Faktor.
    pub fn scale(&mut self, factor: f32) {
        for b in &mut self.buffers {
            for v in b.iter_mut() {
                *v *= factor;
            }
        }
    }

    /// Euklidische Norm ueber alle Parameter.
    pub fn l2_norm(&self) -> f32 {
        let mut acc = 0.0f64;
        for b in &self.buffers {
            for v in b {
                acc += (*v as f64) * (*v as f64);
            }
        }
        acc.sqrt() as f32
    }

    /// Begrenzt die Norm auf `max_norm` und liefert die Norm **vor** der
    /// Begrenzung.
    ///
    /// Ein Skalierungsfaktor statt einer elementweisen Begrenzung, weil sonst
    /// die Richtung des Gradienten veraendert wuerde.
    pub fn clip_by_norm(&mut self, max_norm: f32) -> f32 {
        let norm = self.l2_norm();
        if max_norm > 0.0 && norm > max_norm {
            self.scale(max_norm / norm);
        }
        norm
    }
}

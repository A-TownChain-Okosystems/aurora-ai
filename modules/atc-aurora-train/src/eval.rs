// atc-aurora-train — Bewertung (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Bewertung ist Vorwaertslauf plus Verlust, ohne Gradienten und ohne
// Zustandsaenderung. Drei Zahlen: mittlerer Verlust je Zielposition,
// Perplexitaet und Trefferquote der naechsten Vorhersage. Die Mittelung liegt
// hier und nicht in `cross_entropy`, weil dort die Summe der Vertrag ist
// (B-08) und ein zweiter Mittelungspunkt sonst zwei Definitionen ergaebe.

use atc_aurora_model::config::Config;
use atc_aurora_model::forward::forward;
use atc_aurora_model::weights::Weights;
use atc_aurora_tensor::{cross_entropy, Budget};

use crate::error::TrainError;

/// Ergebnis einer Bewertung.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvalReport {
    /// Anzahl der bewerteten Zielpositionen.
    pub tokens: usize,
    /// Mittlerer Verlust je Zielposition (Kreuzentropie, nats).
    pub mean_loss: f32,
    /// `exp(mean_loss)`. Bei einem Wert ueber `f32::MAX` gekappt.
    pub perplexity: f32,
    /// Anteil der Positionen, an denen `argmax` das Ziel traf.
    pub accuracy: f32,
}

/// Bewertet eine Sammlung von Sequenzen. Jede Sequenz liefert `len - 1`
/// Zielpositionen; eine Sequenz mit weniger als zwei Token ist kein
/// Bewertungsfall und wird abgelehnt.
pub fn evaluate(
    cfg: &Config,
    w: &Weights,
    data: &[Vec<u32>],
    budget_bytes: usize,
) -> Result<EvalReport, TrainError> {
    if data.is_empty() {
        return Err(TrainError::InvalidBatch);
    }
    let mut loss_sum = 0.0f64;
    let mut tokens = 0usize;
    let mut hits = 0usize;

    for seq in data {
        if seq.len() < 2 {
            return Err(TrainError::InvalidBatch);
        }
        let input = &seq[..seq.len() - 1];
        let targets = &seq[1..];
        let mut budget = Budget::new(budget_bytes);
        let logits = forward(cfg, w, input, &mut budget)?;
        let (loss, _grad) = cross_entropy(&logits, targets, &mut budget)?;
        loss_sum += loss.as_slice()[0] as f64;

        // Treffer der naechsten Vorhersage: groesster Logit je Zeile.
        let v = cfg.vocab;
        let rows = logits.shape()[0];
        let src = logits.as_slice();
        for r in 0..rows {
            let row = &src[r * v..(r + 1) * v];
            let mut best = 0usize;
            for (i, value) in row.iter().enumerate() {
                if *value > row[best] {
                    best = i;
                }
            }
            if best == targets[r] as usize {
                hits += 1;
            }
        }
        tokens += targets.len();
    }

    if tokens == 0 {
        return Err(TrainError::InvalidBatch);
    }
    let mean_loss = (loss_sum / tokens as f64) as f32;
    let perplexity_f64 = (loss_sum / tokens as f64).exp();
    // Kappung statt `inf`: ein Perplexitaetswert ueber der Zahlengrenze ist
    // keine Aussage ueber das Modell, sondern ueber f32.
    let perplexity = if perplexity_f64 > f32::MAX as f64 {
        f32::MAX
    } else {
        perplexity_f64 as f32
    };
    Ok(EvalReport {
        tokens,
        mean_loss,
        perplexity,
        accuracy: hits as f32 / tokens as f32,
    })
}

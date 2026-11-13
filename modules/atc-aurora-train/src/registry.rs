// atc-aurora-train — Pruefpunktregister (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Ein Pruefpunkt besteht aus zwei Teilen mit getrennten Zustaendigkeiten:
//
//   * dem Modell als ATCM-Artefakt (`ATC-AI-LLM-002`) mit Pruefsumme aus der
//     Digest-Autoritaet,
//   * dem Trainingszustand (`AURST001`) mit eigener Pruefsumme.
//
// Das Register haelt beide zusammen und ordnet sie nach Schritt. Es ist
// bewusst kein Dateisystem: die Ablage entscheidet der Aufrufer, das Register
// entscheidet nur, welcher Pruefpunkt zu welchem Schritt gehoert und ob seine
// Bytes noch die sind, die geschrieben wurden.

use atc_aurora_model::artifact::{read, write, ReadOptions, TrustAnchor};
use atc_aurora_model::config::Config;
use atc_aurora_model::weights::Weights;
use atc_aurora_tensor::Budget;

use crate::error::TrainError;
use crate::state::{decode, encode, TrainerState};

/// Ein Pruefpunkt: Modellbytes, Zustandsbytes und die Zuordnung zum Schritt.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    /// Abgeschlossene Schritte zum Zeitpunkt der Aufnahme.
    pub step: u64,
    /// Bisher gesehene Zielpositionen.
    pub tokens_seen: u64,
    /// Pruefsumme des Modellartefakts, ausserhalb der Datei gefuehrt.
    pub model_digest: [u8; 32],
    /// Das Modell als ATCM-Artefakt.
    pub model: Vec<u8>,
    /// Der Trainingszustand.
    pub state: Vec<u8>,
}

/// Register der Pruefpunkte eines Laufs.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    entries: Vec<Checkpoint>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            entries: Vec::new(),
        }
    }

    /// Nimmt einen Pruefpunkt auf. Ein Schritt kommt genau einmal vor: zwei
    /// Pruefpunkte mit derselben Nummer waeren zwei verschiedene Zustaende
    /// unter derselben Kennung.
    pub fn register(&mut self, checkpoint: Checkpoint) -> Result<(), TrainError> {
        if self.get(checkpoint.step).is_some() {
            return Err(TrainError::DuplicateCheckpoint {
                step: checkpoint.step,
            });
        }
        self.entries.push(checkpoint);
        self.entries.sort_by_key(|c| c.step);
        Ok(())
    }

    /// Der Pruefpunkt mit der hoechsten Schrittnummer.
    pub fn latest(&self) -> Option<&Checkpoint> {
        self.entries.last()
    }

    /// Pruefpunkt zu einem Schritt.
    pub fn get(&self, step: u64) -> Option<&Checkpoint> {
        self.entries.iter().find(|c| c.step == step)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Alle Schritte in aufsteigender Reihenfolge.
    pub fn steps(&self) -> Vec<u64> {
        self.entries.iter().map(|c| c.step).collect()
    }
}

/// Baut einen Pruefpunkt aus Gewichten und Zustand.
pub fn checkpoint_from(
    cfg: &Config,
    weights: &Weights,
    state: &TrainerState,
) -> Result<Checkpoint, TrainError> {
    let mut model = Vec::new();
    let digest = write(cfg, weights, &mut model).map_err(TrainError::Write)?;
    Ok(Checkpoint {
        step: state.step,
        tokens_seen: state.tokens_seen,
        model_digest: digest,
        model,
        state: encode(state),
    })
}

/// Liest einen Pruefpunkt zurueck: Gewichte und Zustand.
///
/// Der Digest aus dem Pruefpunkt ist der Vertrauensanker des Lesers. Damit ist
/// ein manipuliertes Artefakt kein gueltiger Pruefpunkt, auch wenn seine eigene
/// Pruefsumme nachgerechnet wurde. Der Zustand prueft seine eigene Summe.
pub fn restore(
    cfg: &Config,
    checkpoint: &Checkpoint,
    max_bytes: usize,
) -> Result<(Weights, TrainerState), TrainError> {
    if checkpoint.model.len() > max_bytes {
        return Err(TrainError::Read(
            atc_aurora_model::artifact::ReadError::TooLarge {
                requested: checkpoint.model.len() as u64,
                limit: max_bytes as u64,
            },
        ));
    }
    let mut budget = Budget::new(max_bytes);
    let options = ReadOptions::strict(max_bytes);
    let artifact = read(
        &checkpoint.model,
        TrustAnchor::ExpectedDigest(checkpoint.model_digest),
        &options,
        &mut budget,
        Some(cfg),
    )
    .map_err(TrainError::Read)?;
    let weights = artifact.to_weights().map_err(TrainError::Read)?;

    let param_count = weights.tensors().len();
    let state = decode(&checkpoint.state, param_count)?;
    if state.step != checkpoint.step {
        return Err(TrainError::CorruptState { what: "step" });
    }
    if state.tokens_seen != checkpoint.tokens_seen {
        return Err(TrainError::CorruptState {
            what: "tokens_seen",
        });
    }
    Ok((weights, state))
}

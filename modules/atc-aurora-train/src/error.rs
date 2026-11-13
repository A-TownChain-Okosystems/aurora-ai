// atc-aurora-train — Fehlertypen (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Kein `String` im Fehler, wie im Tensor-Kern: eine Budget- oder
// Pruefsummenabweichung darf nicht daran scheitern, dass die Meldung Speicher
// braucht. Die Varianten tragen nur `&'static str` oder Kopien.

use atc_aurora_model::artifact::{ReadError, WriteError};
use atc_aurora_model::forward::ForwardError;
use atc_aurora_tensor::TensorError;

/// Warum ein Trainingsschritt, eine Bewertung oder ein Pruefpunkt abgelehnt
/// wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainError {
    /// Der Vorwaerts- oder Rueckwaertspfad des Modells hat abgelehnt.
    Forward(ForwardError),
    /// Eine Tensoroperation hat abgelehnt.
    Tensor(TensorError),
    /// Ein Artefakt wurde nicht geschrieben.
    Write(WriteError),
    /// Ein Artefakt wurde nicht gelesen.
    Read(ReadError),
    /// Zahl oder Form der Parameter passt nicht zu den Gradienten.
    WeightMismatch { what: &'static str },
    /// Der Stapel ist leer oder enthaelt eine Sequenz ohne Zielposition.
    InvalidBatch,
    /// Ein Hyperparameter verletzt seinen zulaessigen Bereich.
    InvalidHyperparameter { what: &'static str },
    /// Die Zustandsdatei ist beschaedigt oder passt nicht zu den Parametern.
    CorruptState { what: &'static str },
    /// Fuer diesen Schritt existiert kein Pruefpunkt.
    UnknownCheckpoint { step: u64 },
    /// Ein Pruefpunkt mit diesem Schritt ist bereits registriert.
    DuplicateCheckpoint { step: u64 },
}

impl From<ForwardError> for TrainError {
    fn from(e: ForwardError) -> Self {
        TrainError::Forward(e)
    }
}

impl From<TensorError> for TrainError {
    fn from(e: TensorError) -> Self {
        TrainError::Tensor(e)
    }
}

impl core::fmt::Display for TrainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TrainError::Forward(e) => write!(f, "Forward: {e:?}"),
            TrainError::Tensor(e) => write!(f, "Tensor: {e}"),
            TrainError::Write(e) => write!(f, "Write: {e:?}"),
            TrainError::Read(e) => write!(f, "Read: {e:?}"),
            TrainError::WeightMismatch { what } => write!(f, "WeightMismatch: {what}"),
            TrainError::InvalidBatch => write!(f, "InvalidBatch"),
            TrainError::InvalidHyperparameter { what } => {
                write!(f, "InvalidHyperparameter: {what}")
            }
            TrainError::CorruptState { what } => write!(f, "CorruptState: {what}"),
            TrainError::UnknownCheckpoint { step } => write!(f, "UnknownCheckpoint: {step}"),
            TrainError::DuplicateCheckpoint { step } => write!(f, "DuplicateCheckpoint: {step}"),
        }
    }
}

impl core::error::Error for TrainError {}

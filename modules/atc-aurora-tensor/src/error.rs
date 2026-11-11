// atc-aurora-tensor — Fehlertypen
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.

use core::fmt;

/// Alle Fehler des Tensor-Kerns. Absichtlich ohne String, damit ein Fehler
/// selbst keine Zuteilung auslöst: eine Budgetüberschreitung darf nicht daran
/// scheitern, dass die Fehlermeldung Speicher braucht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorError {
    /// Formen passen nicht zusammen oder Bytes passen nicht zur Form.
    ShapeMismatch,
    /// Operation verlangt einen anderen Rang als der Tensor hat.
    UnsupportedRank { got: usize, expected: &'static str },
    /// Form ist so groß, dass Anzahl oder Bytegröße nicht mehr in `usize` passt.
    SizeOverflow,
    /// Das Budget reicht nicht. Wird vor der Zuteilung zurückgegeben.
    BudgetExceeded { requested: usize, limit: usize },
}

impl fmt::Display for TensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TensorError::ShapeMismatch => write!(f, "ShapeMismatch"),
            TensorError::UnsupportedRank { got, expected } => {
                write!(f, "UnsupportedRank: got {got}, expected {expected}")
            }
            TensorError::SizeOverflow => write!(f, "SizeOverflow"),
            TensorError::BudgetExceeded { requested, limit } => {
                write!(
                    f,
                    "BudgetExceeded: requested {requested} bytes, limit {limit} bytes"
                )
            }
        }
    }
}

impl core::error::Error for TensorError {}

pub type Result<T> = core::result::Result<T, TensorError>;

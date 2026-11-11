// atc-aurora-tensor — nativer Tensor-Kern, M-01, M-02 und M-03
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Eigentümer der KI-Rechenoperationen. Keine Abhängigkeit auf Kernel, Geräte
// oder eine externe Tensor-Bibliothek. f32 ist der verbindliche Datentyp
// (ATC-AI-LLM-001 Abschnitt 7.1.1), Speicher kommt aus `alloc`, transzendente
// Funktionen aus `libm`, damit das Crate ohne std baut.
//
// Jede Speicherzuteilung läuft über ein `Budget` und wird vor der Zuteilung
// geprüft (Testfall B-03).

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod autograd;
pub mod budget;
pub mod error;
pub mod layers;
pub mod loss;
pub mod ops;
pub mod tensor;

pub use autograd::{Graph, NodeId};
pub use budget::Budget;
pub use error::{Result, TensorError};
pub use loss::cross_entropy;
pub use tensor::{Dtype, Tensor, BYTES_PER_ELEMENT, DTYPE};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_autograd;

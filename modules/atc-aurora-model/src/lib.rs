//! M-05 `model`: Konfiguration und Parameterhaushalt der Referenz.
//!
//! Dieser Schnitt beantwortet genau eine Frage mit Rechenweg: wie viele
//! Parameter besitzt die eingefrorene Referenzkonfiguration, und woraus setzen
//! sie sich zusammen. Die Formeln sind aus der Architektur abgeleitet, nicht aus
//! einer Wunschzahl. Weicht ein Sollwert davon ab, ist das ein Befund und keine
//! Rundungsfrage.

#![forbid(unsafe_code)]

pub mod artifact;
pub mod config;
pub mod forward;
pub mod params;
pub mod weights;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_forward;

pub use artifact::{
    read, write, Artifact, ReadError, ReadOptions, TensorDesc, TrustAnchor, WriteError,
};
pub use config::{Config, RefusalReason};
pub use forward::{forward, ForwardError, NORM_EPS, ROPE_BASE};
pub use params::{breakdown, param_count, Breakdown, BreakdownError};
pub use weights::{BlockWeights, SplitMix64, Weights};

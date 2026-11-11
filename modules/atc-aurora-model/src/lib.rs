//! M-05 `model`: Konfiguration und Parameterhaushalt der Referenz.
//!
//! Dieser Schnitt beantwortet genau eine Frage mit Rechenweg: wie viele
//! Parameter besitzt die eingefrorene Referenzkonfiguration, und woraus setzen
//! sie sich zusammen. Die Formeln sind aus der Architektur abgeleitet, nicht aus
//! einer Wunschzahl. Weicht ein Sollwert davon ab, ist das ein Befund und keine
//! Rundungsfrage.

#![forbid(unsafe_code)]

pub mod config;
pub mod params;

#[cfg(test)]
mod tests;

pub use config::{Config, RefusalReason};
pub use params::{breakdown, param_count, Breakdown, BreakdownError};

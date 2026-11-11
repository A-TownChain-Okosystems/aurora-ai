//! Parameterhaushalt: welche Tensoren ein Modell besitzt und wie viele Elemente
//! sie zusammen haben. Keine Zuteilung, keine Initialisierung, nur Zaehlung.

use crate::config::{Config, RefusalReason};

/// Aufschluesselung des Parameterhaushalts in Elementen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breakdown {
    /// Einbettungstabelle `vocab * d_model`.
    pub embedding: usize,
    /// Ausgabeprojektion (nur wenn die Einbettung nicht geteilt wird).
    pub output_projection: usize,
    /// Abfrage-, Schluessel-, Wert- und Ausgabeprojektion je Block, alle Bloecke zusammen.
    pub attention: usize,
    /// Gate, Up und Down je Block, alle Bloecke zusammen.
    pub mlp: usize,
    /// Beide RMSNorm-Gewichte je Block, alle Bloecke zusammen.
    pub block_norms: usize,
    /// Abschliessende RMSNorm.
    pub final_norm: usize,
}

/// Grund, aus dem keine Aufschluesselung gebildet werden kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakdownError {
    /// Die Konfiguration ist unzulaessig.
    Invalid(RefusalReason),
}

/// Bildet die Aufschluesselung. Ueberlauf wird nicht stillschweigend hingenommen:
/// hier wird mit geprueften Operationen gerechnet und im Zweifel abgelehnt.
pub fn breakdown(cfg: &Config) -> Result<Breakdown, BreakdownError> {
    cfg.validate().map_err(BreakdownError::Invalid)?;

    let q = cfg.n_heads * cfg.d_head;
    let kv = cfg.n_kv_heads * cfg.d_head;
    // Ein Block: q, k, v und die Ausgabeprojektion.
    let per_block_attention = q + 2 * kv + cfg.d_model;
    let per_block_mlp = 2 * cfg.d_ff + cfg.d_ff;
    let per_block_norms: usize = 2;

    let attention = per_block_attention
        .checked_mul(cfg.d_model)
        .and_then(|v| v.checked_mul(cfg.n_layers))
        .ok_or(BreakdownError::Invalid(RefusalReason::ZeroDimension))?;
    let mlp = per_block_mlp
        .checked_mul(cfg.d_model)
        .and_then(|v| v.checked_mul(cfg.n_layers))
        .ok_or(BreakdownError::Invalid(RefusalReason::ZeroDimension))?;
    let block_norms = per_block_norms
        .checked_mul(cfg.d_model)
        .and_then(|v| v.checked_mul(cfg.n_layers))
        .ok_or(BreakdownError::Invalid(RefusalReason::ZeroDimension))?;
    let embedding = cfg
        .vocab
        .checked_mul(cfg.d_model)
        .ok_or(BreakdownError::Invalid(RefusalReason::ZeroDimension))?;
    let output_projection = if cfg.tied_embedding { 0 } else { embedding };
    let final_norm = cfg.d_model;

    Ok(Breakdown {
        embedding,
        output_projection,
        attention,
        mlp,
        block_norms,
        final_norm,
    })
}

/// Gesamtzahl der Parameter in Elementen.
pub fn param_count(cfg: &Config) -> Result<usize, BreakdownError> {
    let b = breakdown(cfg)?;
    [
        b.embedding,
        b.output_projection,
        b.attention,
        b.mlp,
        b.block_norms,
        b.final_norm,
    ]
    .iter()
    .try_fold(0usize, |acc, v| acc.checked_add(*v))
    .ok_or(BreakdownError::Invalid(RefusalReason::ZeroDimension))
}

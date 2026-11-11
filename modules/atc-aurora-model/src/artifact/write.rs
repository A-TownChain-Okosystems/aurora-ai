//! Kanonischer Schreibpfad des Artefaktformats.
//!
//! Der Pfad entscheidet nichts: Reihenfolge, Formen und Grenzen kommen aus
//! `cfg` und aus `mod.rs`. Dadurch ist die Ausgabe fuer dieselben Gewichte
//! byte-identisch, und ein erneutes Schreiben eines gelesenen Artefakts
//! aendert nichts (`ATC-AI-LLM-002` Abschnitt 5 und 14, Punkt 6).

use super::{
    data_start, expected_name, put_f32, put_u16, put_u32, put_u64, sha256::Sha256,
    tensor_count_for, WriteError, DTYPE_F32, ENTRY_LEN, E_BYTE_LEN, E_DIM0, E_DIM1, E_NAME,
    E_NAME_LEN, E_OFFSET, E_RANK, FLAG_TIED, FORMAT_VERSION, HEADER_LEN, LAYOUT_ROW_MAJOR, MAGIC,
    MAX_ELEMENTS, MAX_TENSORS, MAX_TENSOR_BYTES, OFF_DATA_BYTES, OFF_DIGEST, OFF_DTYPE, OFF_D_FF,
    OFF_D_HEAD, OFF_D_MODEL, OFF_FLAGS, OFF_LAYOUT, OFF_MAGIC, OFF_MAX_SEQ, OFF_NORM_EPS,
    OFF_N_HEADS, OFF_N_KV_HEADS, OFF_N_LAYERS, OFF_RESERVED, OFF_ROPE_BASE, OFF_TENSOR_COUNT,
    OFF_VERSION, OFF_VOCAB,
};
use crate::config::Config;
use crate::forward::{NORM_EPS, ROPE_BASE};
use crate::weights::Weights;
use atc_aurora_tensor::Tensor;

/// Kanonische Tensorfolge als Sichten auf die Gewichte.
fn canonical<'a>(cfg: &Config, w: &'a Weights) -> Vec<&'a Tensor> {
    let mut list: Vec<&Tensor> =
        Vec::with_capacity(tensor_count_for(cfg.n_layers, cfg.tied_embedding));
    list.push(&w.tok_emb);
    if let Some(head) = &w.lm_head {
        list.push(head);
    }
    for b in &w.blocks {
        list.push(&b.attn_norm);
        list.push(&b.wq);
        list.push(&b.wk);
        list.push(&b.wv);
        list.push(&b.wo);
        list.push(&b.mlp_norm);
        list.push(&b.wgu);
        list.push(&b.wd);
    }
    list.push(&w.final_norm);
    list
}

/// Prueft die Gewichte gegen die Konfiguration, ohne zu schreiben.
fn check(cfg: &Config, w: &Weights) -> Result<(), WriteError> {
    let d = cfg.d_model;
    let q = cfg.n_heads * cfg.d_head;
    let kv = cfg.n_kv_heads * cfg.d_head;
    if w.tok_emb.shape() != [cfg.vocab, d] {
        return Err(WriteError::WeightMismatch { what: "tok_emb" });
    }
    if w.final_norm.shape() != [d] {
        return Err(WriteError::WeightMismatch { what: "final_norm" });
    }
    if w.blocks.len() != cfg.n_layers {
        return Err(WriteError::WeightMismatch { what: "blocks" });
    }
    match (&w.lm_head, cfg.tied_embedding) {
        (None, true) => {}
        (Some(head), false) if head.shape() == [d, cfg.vocab] => {}
        _ => return Err(WriteError::WeightMismatch { what: "lm_head" }),
    }
    for b in &w.blocks {
        let ok = b.attn_norm.shape() == [d]
            && b.mlp_norm.shape() == [d]
            && b.wq.shape() == [d, q]
            && b.wk.shape() == [d, kv]
            && b.wv.shape() == [d, kv]
            && b.wo.shape() == [q, d]
            && b.wgu.shape() == [d, 2 * cfg.d_ff]
            && b.wd.shape() == [cfg.d_ff, d];
        if !ok {
            return Err(WriteError::WeightMismatch { what: "block" });
        }
    }
    Ok(())
}

/// Schreibt ein Artefakt und liefert den Digest zurueck.
///
/// Der Digest wird **zuletzt** berechnet, ueber die gesamte Datei mit
/// genulltem Digest-Feld. Der Rueckgabewert ist der Vertrauensanker fuer
/// `read`: er gehoert ausserhalb der Datei abgelegt, sonst belegt der Digest
/// nur die Unveraendertheit, nicht die Herkunft.
pub fn write(cfg: &Config, w: &Weights, out: &mut Vec<u8>) -> Result<[u8; 32], WriteError> {
    cfg.validate().map_err(WriteError::InvalidConfig)?;
    check(cfg, w)?;

    let tensors = canonical(cfg, w);
    let count = tensors.len();
    if count != tensor_count_for(cfg.n_layers, cfg.tied_embedding) || count > MAX_TENSORS {
        return Err(WriteError::WeightMismatch {
            what: "tensor_count",
        });
    }

    let mut data_bytes: u64 = 0;
    let mut lengths = Vec::with_capacity(count);
    for (index, t) in tensors.iter().enumerate() {
        let elements = t.size() as u64;
        if elements > MAX_ELEMENTS {
            return Err(WriteError::TooLarge {
                requested: elements,
                limit: MAX_ELEMENTS,
            });
        }
        let bytes = elements
            .checked_mul(super::ELEMENT_LEN)
            .ok_or(WriteError::ShapeOverflow { index })?;
        data_bytes = data_bytes
            .checked_add(bytes)
            .ok_or(WriteError::ShapeOverflow { index })?;
        lengths.push(bytes);
    }
    if data_bytes > MAX_TENSOR_BYTES {
        return Err(WriteError::TooLarge {
            requested: data_bytes,
            limit: MAX_TENSOR_BYTES,
        });
    }

    let start = data_start(count) as u64;
    let total = start
        .checked_add(data_bytes)
        .ok_or(WriteError::ShapeOverflow { index: 0 })?;
    let total = usize::try_from(total).map_err(|_| WriteError::TooLarge {
        requested: total,
        limit: usize::MAX as u64,
    })?;

    out.clear();
    out.resize(total, 0);

    // Header
    out[OFF_MAGIC..OFF_MAGIC + 4].copy_from_slice(&MAGIC);
    put_u16(out, OFF_VERSION, FORMAT_VERSION);
    put_u16(
        out,
        OFF_FLAGS,
        if cfg.tied_embedding { FLAG_TIED } else { 0 },
    );
    put_u32(out, OFF_D_MODEL, cfg.d_model as u32);
    put_u32(out, OFF_N_LAYERS, cfg.n_layers as u32);
    put_u32(out, OFF_N_HEADS, cfg.n_heads as u32);
    put_u32(out, OFF_N_KV_HEADS, cfg.n_kv_heads as u32);
    put_u32(out, OFF_D_HEAD, cfg.d_head as u32);
    put_u32(out, OFF_D_FF, cfg.d_ff as u32);
    put_u32(out, OFF_VOCAB, cfg.vocab as u32);
    put_u32(out, OFF_MAX_SEQ, cfg.max_seq as u32);
    put_u32(out, OFF_TENSOR_COUNT, count as u32);
    put_u32(out, OFF_DTYPE, DTYPE_F32);
    put_u32(out, OFF_LAYOUT, LAYOUT_ROW_MAJOR);
    put_f32(out, OFF_ROPE_BASE, ROPE_BASE);
    put_f32(out, OFF_NORM_EPS, NORM_EPS);
    put_u32(out, OFF_RESERVED, 0);
    put_u64(out, OFF_DATA_BYTES, data_bytes);

    // Tabelle und Daten
    let mut offset: u64 = 0;
    let mut cursor = data_start(count);
    for (index, t) in tensors.iter().enumerate() {
        let entry = HEADER_LEN + ENTRY_LEN * index;
        let mut name = [0u8; 32];
        let name_len = expected_name(index, cfg.tied_embedding, cfg.n_layers, &mut name)
            .ok_or(WriteError::NameTooLong { index })?;
        put_u32(out, entry + E_NAME_LEN, name_len as u32);
        out[entry + E_NAME..entry + E_NAME + 32].copy_from_slice(&name);
        let shape = t.shape();
        let (rank, dim0, dim1) = match shape {
            [a] => (1u32, *a as u32, 1u32),
            [a, b] => (2u32, *a as u32, *b as u32),
            _ => return Err(WriteError::ShapeOverflow { index }),
        };
        put_u32(out, entry + E_RANK, rank);
        put_u32(out, entry + E_DIM0, dim0);
        put_u32(out, entry + E_DIM1, dim1);
        put_u64(out, entry + E_OFFSET, offset);
        put_u64(out, entry + E_BYTE_LEN, lengths[index]);

        for (i, value) in t.as_slice().iter().enumerate() {
            if !value.is_finite() {
                return Err(WriteError::ValueNotFinite { index });
            }
            let at = cursor + i * 4;
            out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        offset += lengths[index];
        cursor += lengths[index] as usize;
    }

    // Digest zuletzt, ueber die Datei mit genulltem Digest-Feld (es ist null).
    let digest = Sha256::new().tap(out).finish();
    out[OFF_DIGEST..OFF_DIGEST + 32].copy_from_slice(&digest);
    Ok(digest)
}

/// Kleiner Helfer, damit der Digest in einem Ausdruck entsteht.
trait Tap: Sized {
    fn tap(self, bytes: &[u8]) -> Self;
}

impl Tap for Sha256 {
    fn tap(mut self, bytes: &[u8]) -> Self {
        self.update(bytes);
        self
    }
}

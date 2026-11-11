//! Strikter Lesepfad des Artefaktformats.
//!
//! Reihenfolge und Grenzen folgen `ATC-AI-LLM-002` Abschnitt 7 und 8. Zwei
//! Eigenschaften sind nicht verhandelbar:
//!
//! * **Pruefen vor Zuteilen.** Keine Allokation anhand eines Werts aus der
//!   Datei; `max_bytes` und die harten Grenzen greifen zuerst, der Digest wird
//!   erst nach erfolgreicher Strukturpruefung berechnet.
//! * **Struktur vor Vertrauen.** Ist die Datei strukturell falsch und der
//!   erwartete Digest ebenfalls, antwortet der Leser mit dem Strukturfehler
//!   (Abschnitt 7, Praezedenz).

use super::{
    check_name_field, data_start, expected_name, f32_at, parse_header, sha256::Sha256, u32_at,
    u64_at, ReadError, DTYPE_F32, ENTRY_LEN, E_BYTE_LEN, E_DIM0, E_DIM1, E_NAME, E_NAME_LEN,
    E_OFFSET, E_RANK, FLAG_TIED, FORMAT_VERSION, HEADER_LEN, LAYOUT_ROW_MAJOR, MAX_DIM,
    MAX_ELEMENTS, MAX_TENSORS, MAX_TENSOR_BYTES, OFF_DIGEST, OFF_NORM_EPS, OFF_ROPE_BASE,
};
use crate::config::Config;
use crate::forward::{NORM_EPS, ROPE_BASE};
use crate::weights::{BlockWeights, Weights};
use atc_aurora_tensor::{Budget, Tensor};

/// Vertrauensanker fuer einen Lesevorgang.
///
/// Der Digest in der Datei belegt nur, dass die Bytes seit dem Schreiben
/// unveraendert sind. Wer manipulieren kann, rechnet ihn neu. Vertrauen
/// entsteht deshalb nur durch eine Angabe **ausserhalb** der Datei.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustAnchor {
    /// Kein Anker. Das Ergebnis ist formal gueltig, aber nicht vertrauenswuerdig.
    None,
    /// Der erwartete Digest, aus einer unabhaengigen Quelle.
    ExpectedDigest([u8; 32]),
    /// Der Aufrufer hat eine Signatur ueber den Digest bereits geprueft.
    ///
    /// Der Leser kann diese Behauptung **nicht** nachpruefen; bis zur
    /// Entscheidung K7 (Signaturverfahren) ist dieser Zweig fuer Aufrufer
    /// gedacht, die selbst verifizieren. Er ist kein Ersatz fuer K7.
    SignatureVerified,
}

/// Grenzen und Modus eines Lesevorgangs. `max_bytes` ist Pflicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadOptions {
    /// Obergrenze fuer die Dateilaenge. Wird **vor** dem Hashen geprueft.
    pub max_bytes: usize,
    /// Ohne Anker ablehnen (`TrustAnchorMissing`) statt nur zu markieren.
    pub strict_anchor: bool,
    /// Zusaetzlich jedes Gewicht auf Endlichkeit pruefen.
    pub check_finite: bool,
}

impl ReadOptions {
    /// Strenger Modus: Ankerpflicht, keine Endlichkeitspruefung.
    pub const fn strict(max_bytes: usize) -> Self {
        ReadOptions {
            max_bytes,
            strict_anchor: true,
            check_finite: false,
        }
    }

    /// Ohne Ankerpflicht; das Ergebnis traegt `trustworthy = false`.
    pub const fn lenient(max_bytes: usize) -> Self {
        ReadOptions {
            max_bytes,
            strict_anchor: false,
            check_finite: false,
        }
    }

    /// Zusaetzliche Endlichkeitspruefung.
    pub const fn with_finite_check(mut self) -> Self {
        self.check_finite = true;
        self
    }
}

/// Ein Tensoreintrag, wie er in der Datei steht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TensorDesc {
    name: [u8; 32],
    name_len: u8,
    rank: u8,
    dim0: u32,
    dim1: u32,
    offset: u64,
    byte_len: u64,
}

impl TensorDesc {
    /// Name als Text (die Zeichenregel wurde beim Lesen geprueft).
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("")
    }

    /// Rang des Tensors.
    pub const fn rank(&self) -> u8 {
        self.rank
    }

    /// Form als zwei Dimensionen; bei Rang 1 ist `dim1` = 1.
    pub const fn dims(&self) -> (u32, u32) {
        (self.dim0, self.dim1)
    }

    /// Elementzahl.
    pub const fn elements(&self) -> u64 {
        self.dim0 as u64 * self.dim1 as u64
    }

    /// Start im Datenbereich.
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Laenge in Byte.
    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }
}

/// Gelesenes Artefakt. Haelt nur Sichten und Beschreibungen, keine Gewichtskopie.
#[derive(Debug)]
pub struct Artifact<'a> {
    bytes: &'a [u8],
    config: Config,
    digest: [u8; 32],
    trustworthy: bool,
    rope_base: f32,
    norm_eps: f32,
    tensors: Vec<TensorDesc>,
}

impl<'a> Artifact<'a> {
    /// Architektur, wie sie in der Datei steht (und geprueft wurde).
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Digest ueber den gesamten Inhalt (Digest-Feld genullt).
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Nur `true`, wenn ein Anker den Inhalt bestaetigt hat.
    pub const fn trustworthy(&self) -> bool {
        self.trustworthy
    }

    /// `rope_base` aus dem Artefakt.
    pub const fn rope_base(&self) -> f32 {
        self.rope_base
    }

    /// `norm_eps` aus dem Artefakt.
    pub const fn norm_eps(&self) -> f32 {
        self.norm_eps
    }

    /// Alle Tensorbeschreibungen in kanonischer Reihenfolge.
    pub fn tensors(&self) -> &[TensorDesc] {
        &self.tensors
    }

    /// Rohbytes eines Tensors als Sicht auf die Eingabe.
    pub fn tensor_bytes(&self, index: usize) -> Option<&'a [u8]> {
        let desc = self.tensors.get(index)?;
        let start = data_start(self.tensors.len()) + desc.offset as usize;
        let end = start + desc.byte_len as usize;
        self.bytes.get(start..end)
    }

    /// Ein Tensor als `f32`-Vektor.
    ///
    /// Das ist eine bewusste Kopie **eines** Tensors, kein Modellabbild: die
    /// Sichten aus `tensor_bytes` sind byteweise und duerfen nicht als
    /// ausgerichtete `f32`-Folge interpretiert werden (Befund F-04).
    pub fn tensor_f32(&self, index: usize) -> Option<Vec<f32>> {
        let bytes = self.tensor_bytes(index)?;
        let mut out = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.as_chunks::<4>().0 {
            out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Some(out)
    }

    /// Baut die Gewichte. Dies ist die **explizite** Kopie des Modells und
    /// liegt ausserhalb des Lesers; er selbst hat nur Beschreibungen angelegt.
    pub fn to_weights(&self) -> Result<Weights, ReadError> {
        let cfg = &self.config;
        let d = cfg.d_model;
        let q = cfg.n_heads * cfg.d_head;
        let kv = cfg.n_kv_heads * cfg.d_head;
        let mut cursor = 0usize;

        let mut next = |dims: &[usize]| -> Result<Tensor, ReadError> {
            let data = self
                .tensor_f32(cursor)
                .ok_or(ReadError::ShapeInvalid { index: cursor })?;
            let index = cursor;
            cursor += 1;
            Tensor::new(dims.to_vec(), data).map_err(|_| ReadError::ShapeInvalid { index })
        };

        let tok_emb = next(&[cfg.vocab, d])?;
        let lm_head = if cfg.tied_embedding {
            None
        } else {
            Some(next(&[d, cfg.vocab])?)
        };
        let mut blocks = Vec::with_capacity(cfg.n_layers);
        for _ in 0..cfg.n_layers {
            blocks.push(BlockWeights {
                attn_norm: next(&[d])?,
                wq: next(&[d, q])?,
                wk: next(&[d, kv])?,
                wv: next(&[d, kv])?,
                wo: next(&[q, d])?,
                mlp_norm: next(&[d])?,
                wgu: next(&[d, 2 * cfg.d_ff])?,
                wd: next(&[cfg.d_ff, d])?,
            });
        }
        let final_norm = next(&[d])?;
        Ok(Weights {
            tok_emb,
            lm_head,
            blocks,
            final_norm,
        })
    }
}

/// Liest und prueft ein Artefakt nach `ATC-AI-LLM-002` v1.1.0.
pub fn read<'a>(
    bytes: &'a [u8],
    anchor: TrustAnchor,
    options: &ReadOptions,
    budget: &mut Budget,
    expected_cfg: Option<&Config>,
) -> Result<Artifact<'a>, ReadError> {
    // 1. Grundlaenge und Grenze des Aufrufers, ohne zu hashen.
    if bytes.len() < HEADER_LEN {
        return Err(ReadError::Truncated { at: bytes.len() });
    }
    if bytes.len() > options.max_bytes {
        return Err(ReadError::TooLarge {
            requested: bytes.len() as u64,
            limit: options.max_bytes as u64,
        });
    }

    // 2. Kennung, Version, Flags, reservierte Felder, Typ, Layout.
    let header = parse_header(bytes)?;
    if header.format_version != FORMAT_VERSION {
        return Err(ReadError::UnsupportedVersion {
            got: header.format_version,
            expected: FORMAT_VERSION,
        });
    }
    if header.flags & !FLAG_TIED != 0 {
        return Err(ReadError::UnknownFlag {
            bits: header.flags & !FLAG_TIED,
        });
    }
    if header.reserved != 0 {
        return Err(ReadError::ReservedNotZero);
    }
    if header.dtype != DTYPE_F32 {
        return Err(ReadError::UnsupportedDtype { got: header.dtype });
    }
    if header.layout != LAYOUT_ROW_MAJOR {
        return Err(ReadError::UnsupportedLayout { got: header.layout });
    }

    // 3. Architekturparameter.
    let cfg = config_from_header(&header)?;
    cfg.validate()
        .map_err(|_| ReadError::InvalidArgument { field: "config" })?;
    let expected_count = header.expected_tensor_count();
    if header.tensor_count as usize != expected_count {
        return Err(ReadError::TensorCountMismatch {
            got: header.tensor_count as usize,
            expected: expected_count,
        });
    }
    if expected_count > MAX_TENSORS {
        return Err(ReadError::TooLarge {
            requested: expected_count as u64,
            limit: MAX_TENSORS as u64,
        });
    }

    let rope_base = f32_at(bytes, OFF_ROPE_BASE)?;
    let norm_eps = f32_at(bytes, OFF_NORM_EPS)?;
    if !rope_base.is_finite() || rope_base <= 0.0 {
        return Err(ReadError::InvalidArgument { field: "rope_base" });
    }
    if !norm_eps.is_finite() || norm_eps <= 0.0 {
        return Err(ReadError::InvalidArgument { field: "norm_eps" });
    }
    // Der Rechenkern kennt nur seine eigenen Konstanten (K6). Ein Artefakt mit
    // anderer Numerik wird abgelehnt statt still falsch gerechnet.
    if rope_base != ROPE_BASE {
        return Err(ReadError::ArchMismatch { field: "rope_base" });
    }
    if norm_eps != NORM_EPS {
        return Err(ReadError::ArchMismatch { field: "norm_eps" });
    }

    // 4. Dateilaenge exakt.
    if header.tensor_data_bytes > MAX_TENSOR_BYTES {
        return Err(ReadError::TooLarge {
            requested: header.tensor_data_bytes,
            limit: MAX_TENSOR_BYTES,
        });
    }
    let start = data_start(expected_count) as u64;
    let expected_len = start
        .checked_add(header.tensor_data_bytes)
        .ok_or(ReadError::TooLarge {
            requested: u64::MAX,
            limit: u64::MAX,
        })?;
    let file_len = bytes.len() as u64;
    if file_len < expected_len {
        return Err(ReadError::LengthMismatch {
            file: bytes.len(),
            expected: expected_len,
        });
    }
    if file_len > expected_len {
        return Err(ReadError::TrailingBytes {
            extra: file_len - expected_len,
        });
    }

    // 5. Budget des Aufrufers, bevor Beschreibungen entstehen.
    let desc_bytes = expected_count
        .checked_mul(core::mem::size_of::<TensorDesc>())
        .ok_or(ReadError::TooLarge {
            requested: u64::MAX,
            limit: budget.limit() as u64,
        })?;
    budget
        .reserve(desc_bytes)
        .map_err(|_| ReadError::BudgetExceeded {
            requested: desc_bytes as u64,
            limit: budget.limit() as u64,
        })?;

    // 6. Tabelle: Namen, Formen, Laengen, lueckenlose Offsets.
    let mut tensors = Vec::with_capacity(expected_count);
    let mut running: u64 = 0;
    let tied = header.tied();
    for index in 0..expected_count {
        let entry = HEADER_LEN + ENTRY_LEN * index;
        let name_len = u32_at(bytes, entry + E_NAME_LEN)? as usize;
        if name_len == 0 || name_len > 32 {
            return Err(ReadError::InvalidName { index });
        }
        let mut field = [0u8; 32];
        field.copy_from_slice(&bytes[entry + E_NAME..entry + E_NAME + 32]);
        if !check_name_field(&field, name_len) {
            return Err(ReadError::InvalidName { index });
        }
        let mut expected = [0u8; 32];
        let expected_len = expected_name(index, tied, header.n_layers as usize, &mut expected)
            .ok_or(ReadError::TensorOrderInvalid { index })?;
        if expected_len != name_len || expected != field {
            return Err(ReadError::TensorOrderInvalid { index });
        }

        let rank = u32_at(bytes, entry + E_RANK)?;
        let dim0 = u32_at(bytes, entry + E_DIM0)?;
        let dim1 = u32_at(bytes, entry + E_DIM1)?;
        let offset = u64_at(bytes, entry + E_OFFSET)?;
        let byte_len = u64_at(bytes, entry + E_BYTE_LEN)?;

        if dim0 == 0 || dim0 > MAX_DIM || dim1 == 0 || dim1 > MAX_DIM {
            return Err(ReadError::ShapeInvalid { index });
        }
        if !matches!(rank, 1 | 2) || (rank == 1 && dim1 != 1) {
            return Err(ReadError::ShapeInvalid { index });
        }
        let (want0, want1) = expected_dims(&cfg, index);
        if dim0 != want0 || dim1 != want1 {
            return Err(ReadError::ShapeInvalid { index });
        }
        let elements = (dim0 as u64)
            .checked_mul(dim1 as u64)
            .ok_or(ReadError::ShapeOverflow { index })?;
        if elements > MAX_ELEMENTS {
            return Err(ReadError::ShapeInvalid { index });
        }
        let want_bytes = elements * 4;
        if byte_len != want_bytes {
            return Err(ReadError::ByteLenMismatch {
                index,
                declared: byte_len,
                expected: want_bytes,
            });
        }
        if offset != running {
            return Err(ReadError::OffsetNotContiguous { index });
        }
        running = running
            .checked_add(byte_len)
            .ok_or(ReadError::ShapeOverflow { index })?;

        tensors.push(TensorDesc {
            name: field,
            name_len: name_len as u8,
            rank: rank as u8,
            dim0,
            dim1,
            offset,
            byte_len,
        });
    }
    if running != header.tensor_data_bytes {
        return Err(ReadError::OffsetNotContiguous {
            index: expected_count,
        });
    }

    // 7. Digest ueber die ganze Datei mit genulltem Digest-Feld.
    let mut hasher = Sha256::new();
    hasher.update(&bytes[..OFF_DIGEST]);
    hasher.update(&[0u8; 32]);
    hasher.update(&bytes[OFF_DIGEST + 32..]);
    let actual = hasher.finish();
    let stored = {
        let mut d = [0u8; 32];
        d.copy_from_slice(&bytes[OFF_DIGEST..OFF_DIGEST + 32]);
        d
    };
    if stored != actual {
        return Err(ReadError::DigestMismatch {
            expected: stored,
            actual,
        });
    }

    // 8. Anker.
    let trustworthy = match anchor {
        TrustAnchor::None => {
            if options.strict_anchor {
                return Err(ReadError::TrustAnchorMissing);
            }
            false
        }
        TrustAnchor::ExpectedDigest(expected) => {
            if expected != actual {
                return Err(ReadError::DigestMismatch { expected, actual });
            }
            true
        }
        TrustAnchor::SignatureVerified => true,
    };

    // 9. Erwartete Architektur, falls der Aufrufer eine vorgibt.
    if let Some(want) = expected_cfg {
        compare_config(want, &cfg)?;
    }

    // 10. Endlichkeit, optional.
    if options.check_finite {
        let artifact = Artifact {
            bytes,
            config: cfg,
            digest: actual,
            trustworthy,
            rope_base,
            norm_eps,
            tensors,
        };
        for index in 0..artifact.tensors.len() {
            let values = artifact
                .tensor_f32(index)
                .ok_or(ReadError::ShapeInvalid { index })?;
            if values.iter().any(|v| !v.is_finite()) {
                return Err(ReadError::ValueNotFinite { index });
            }
        }
        return Ok(artifact);
    }

    Ok(Artifact {
        bytes,
        config: cfg,
        digest: actual,
        trustworthy,
        rope_base,
        norm_eps,
        tensors,
    })
}

fn config_from_header(h: &super::Header) -> Result<Config, ReadError> {
    let u = |value: u32, field: &'static str| -> Result<usize, ReadError> {
        if value == 0 || value > MAX_DIM {
            return Err(ReadError::InvalidArgument { field });
        }
        Ok(value as usize)
    };
    Ok(Config {
        d_model: u(h.d_model, "d_model")?,
        n_layers: u(h.n_layers, "n_layers")?,
        n_heads: u(h.n_heads, "n_heads")?,
        n_kv_heads: u(h.n_kv_heads, "n_kv_heads")?,
        d_head: u(h.d_head, "d_head")?,
        d_ff: u(h.d_ff, "d_ff")?,
        vocab: u(h.vocab, "vocab")?,
        max_seq: u(h.max_seq, "max_seq")?,
        tied_embedding: h.tied(),
    })
}

/// Erwartete Form eines Tensors an kanonischer Position.
fn expected_dims(cfg: &Config, index: usize) -> (u32, u32) {
    let d = cfg.d_model as u32;
    let q = (cfg.n_heads * cfg.d_head) as u32;
    let kv = (cfg.n_kv_heads * cfg.d_head) as u32;
    let block_start = if cfg.tied_embedding { 1 } else { 2 };
    if index == 0 {
        return (cfg.vocab as u32, d);
    }
    if !cfg.tied_embedding && index == 1 {
        return (d, cfg.vocab as u32);
    }
    let block_end = block_start + 8 * cfg.n_layers;
    if index >= block_start && index < block_end {
        return match (index - block_start) % 8 {
            0 => (d, 1),
            1 => (d, q),
            2 => (d, kv),
            3 => (d, kv),
            4 => (q, d),
            5 => (d, 1),
            6 => (d, 2 * cfg.d_ff as u32),
            _ => (cfg.d_ff as u32, d),
        };
    }
    let _ = block_end;
    (d, 1)
}

fn compare_config(want: &Config, got: &Config) -> Result<(), ReadError> {
    let checks: [(&'static str, usize, usize); 8] = [
        ("d_model", want.d_model, got.d_model),
        ("n_layers", want.n_layers, got.n_layers),
        ("n_heads", want.n_heads, got.n_heads),
        ("n_kv_heads", want.n_kv_heads, got.n_kv_heads),
        ("d_head", want.d_head, got.d_head),
        ("d_ff", want.d_ff, got.d_ff),
        ("vocab", want.vocab, got.vocab),
        ("max_seq", want.max_seq, got.max_seq),
    ];
    for (field, a, b) in checks {
        if a != b {
            return Err(ReadError::ArchMismatch { field });
        }
    }
    if want.tied_embedding != got.tied_embedding {
        return Err(ReadError::ArchMismatch {
            field: "tied_embedding",
        });
    }
    Ok(())
}

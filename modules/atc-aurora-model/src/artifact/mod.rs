//! Artefaktvertrag `ATCM` v1: Layout, Grenzen, Fehler und Lesezugriff.
//!
//! Verbindliche Grundlage ist `ATC-AI-LLM-002` Fassung 1.1.0. Dieses Modul
//! enthaelt **keine** freie Entscheidung ueber Feldpositionen: alle Offsets
//! stehen hier einmal als Konstanten, und `write` wie `read` benutzen
//! ausschliesslich diese Konstanten. Damit kann die Feldtabelle des Vertrags
//! nicht wieder von der Implementierung abweichen (Befund F-01 der Gate-Pruefung).

pub mod read;
pub mod sha256;
pub mod write;

#[cfg(test)]
mod tests;

pub use read::{read, Artifact, ReadOptions, TensorDesc, TrustAnchor};
pub use sha256::Sha256;
pub use write::write;

use crate::config::RefusalReason;

/// Kennung im Header.
pub const MAGIC: [u8; 4] = *b"ATCM";
/// Einzige zulaessige Formatversion.
pub const FORMAT_VERSION: u16 = 1;
/// Flag: Einbettung und Ausgabekopf sind gebunden.
pub const FLAG_TIED: u16 = 1 << 0;
/// Datentyp f32 (IEEE 754, little endian).
pub const DTYPE_F32: u32 = 0;
/// Layout: zeilenweise, dicht.
pub const LAYOUT_ROW_MAJOR: u32 = 0;

/// Headerlaenge in Byte: 4+2+2+44+4+4+4+8+32 = 104.
pub const HEADER_LEN: usize = 104;
/// Laenge eines Tabelleneintrags.
pub const ENTRY_LEN: usize = 64;
/// Groesse eines f32 in Byte.
pub const ELEMENT_LEN: u64 = 4;

/// Harte Grenze fuer den Tensor-Datenbereich (1 GiB).
pub const MAX_TENSOR_BYTES: u64 = 1 << 30;
/// Harte Grenze fuer die Zahl der Tensoren: 2 + 8·1024.
pub const MAX_TENSORS: usize = 8194;
/// Harte Grenze fuer eine einzelne Dimension.
pub const MAX_DIM: u32 = 1 << 20;
/// Harte Grenze fuer Elemente je Tensor.
pub const MAX_ELEMENTS: u64 = 1 << 28;

// Feldpositionen im Header. Einzige Quelle fuer beide Pfade.
pub const OFF_MAGIC: usize = 0;
pub const OFF_VERSION: usize = 4;
pub const OFF_FLAGS: usize = 6;
pub const OFF_D_MODEL: usize = 8;
pub const OFF_N_LAYERS: usize = 12;
pub const OFF_N_HEADS: usize = 16;
pub const OFF_N_KV_HEADS: usize = 20;
pub const OFF_D_HEAD: usize = 24;
pub const OFF_D_FF: usize = 28;
pub const OFF_VOCAB: usize = 32;
pub const OFF_MAX_SEQ: usize = 36;
pub const OFF_TENSOR_COUNT: usize = 40;
pub const OFF_DTYPE: usize = 44;
pub const OFF_LAYOUT: usize = 48;
pub const OFF_ROPE_BASE: usize = 52;
pub const OFF_NORM_EPS: usize = 56;
pub const OFF_RESERVED: usize = 60;
pub const OFF_DATA_BYTES: usize = 64;
pub const OFF_DIGEST: usize = 72;

// Feldpositionen im Tabelleneintrag.
pub const E_NAME_LEN: usize = 0;
pub const E_NAME: usize = 4;
pub const E_RANK: usize = 36;
pub const E_DIM0: usize = 40;
pub const E_DIM1: usize = 44;
pub const E_OFFSET: usize = 48;
pub const E_BYTE_LEN: usize = 56;

/// Namen der acht Tensoren eines Blocks, in kanonischer Reihenfolge.
pub const BLOCK_SUFFIXES: [&str; 8] =
    ["attn_norm", "wq", "wk", "wv", "wo", "mlp_norm", "wgu", "wd"];

/// Erwartete Tensorzahl zu einer Architektur.
pub const fn tensor_count_for(n_layers: usize, tied: bool) -> usize {
    2 + 8 * n_layers + if tied { 0 } else { 1 }
}

/// Beginnt der Tensor-Datenbereich.
pub const fn data_start(tensor_count: usize) -> usize {
    HEADER_LEN + ENTRY_LEN * tensor_count
}

/// Architekturparameter, wie sie im Header stehen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub format_version: u16,
    pub flags: u16,
    pub d_model: u32,
    pub n_layers: u32,
    pub n_heads: u32,
    pub n_kv_heads: u32,
    pub d_head: u32,
    pub d_ff: u32,
    pub vocab: u32,
    pub max_seq: u32,
    pub tensor_count: u32,
    pub dtype: u32,
    pub layout: u32,
    pub reserved: u32,
    pub tensor_data_bytes: u64,
}

impl Header {
    /// Bindet die Einbettung?
    pub const fn tied(&self) -> bool {
        self.flags & FLAG_TIED != 0
    }

    /// Erwartete Tensorzahl nach diesem Header.
    pub const fn expected_tensor_count(&self) -> usize {
        tensor_count_for(self.n_layers as usize, self.tied())
    }
}

/// Warum ein Artefakt nicht gelesen wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// Datei endet vor dem angeforderten Feld.
    Truncated { at: usize },
    /// Kennung ist nicht `ATCM`.
    BadMagic,
    /// Formatversion ist nicht 1.
    UnsupportedVersion { got: u16, expected: u16 },
    /// Datentyp ist unbekannt.
    UnsupportedDtype { got: u32 },
    /// Layout ist unbekannt.
    UnsupportedLayout { got: u32 },
    /// Reserviertes Flag gesetzt.
    UnknownFlag { bits: u16 },
    /// `reserved` ist nicht 0.
    ReservedNotZero,
    /// Tabelle passt nicht zur Architektur.
    TensorCountMismatch { got: usize, expected: usize },
    /// Name verletzt die Zeichenregel oder ist nicht NUL-aufgefuellt.
    InvalidName { index: usize },
    /// Reihenfolge ist nicht kanonisch.
    TensorOrderInvalid { index: usize },
    /// Dimension ist 0, zu gross oder `dim1` bei Rang 1 nicht 1.
    ShapeInvalid { index: usize },
    /// Produkt der Dimensionen laeuft ueber.
    ShapeOverflow { index: usize },
    /// `byte_len` passt nicht zum Formprodukt.
    ByteLenMismatch {
        index: usize,
        declared: u64,
        expected: u64,
    },
    /// Offsets sind nicht lueckenlos und ueberlappend.
    OffsetNotContiguous { index: usize },
    /// Datei ist kuerzer als der Header verlangt.
    LengthMismatch { file: usize, expected: u64 },
    /// Bytes hinter dem Datenbereich.
    TrailingBytes { extra: u64 },
    /// Ueber einer festen Grenze.
    TooLarge { requested: u64, limit: u64 },
    /// Ueber dem Budget des Aufrufers.
    BudgetExceeded { requested: u64, limit: u64 },
    /// Pruefsumme passt nicht zum Inhalt.
    DigestMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    /// Kein Vertrauensanker, strenger Modus.
    TrustAnchorMissing,
    /// Parameter weicht von der erwarteten Konfiguration ab.
    ArchMismatch { field: &'static str },
    /// Gewicht ist `NaN` oder `inf` (nur im strengen Modus).
    ValueNotFinite { index: usize },
    /// Feld ist inhaltlich unbrauchbar (z. B. `rope_base` = 0).
    InvalidArgument { field: &'static str },
}

/// Warum ein Artefakt nicht geschrieben wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// Konfiguration ist unzulaessig.
    InvalidConfig(RefusalReason),
    /// Gewichte passen nicht zur Konfiguration.
    WeightMismatch { what: &'static str },
    /// Ein Name waere laenger als das Feld.
    NameTooLong { index: usize },
    /// Ein Produkt laeuft ueber.
    ShapeOverflow { index: usize },
    /// Ueber einer festen Grenze.
    TooLarge { requested: u64, limit: u64 },
    /// Gewicht ist `NaN` oder `inf`.
    ValueNotFinite { index: usize },
}

// --- Lesen und Schreiben einzelner Felder -----------------------------------
// Alle Felder werden byteweise konvertiert. Es gibt keine Ausrichtungsannahme
// und keinen Cast zwischen Byte- und Zahlenpuffer (Befund F-04).

fn need(bytes: &[u8], at: usize, len: usize) -> Result<(), ReadError> {
    if at.checked_add(len).map(|end| end <= bytes.len()) == Some(true) {
        Ok(())
    } else {
        Err(ReadError::Truncated { at })
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, ReadError> {
    need(bytes, at, 2)?;
    let mut b = [0u8; 2];
    b.copy_from_slice(&bytes[at..at + 2]);
    Ok(u16::from_le_bytes(b))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, ReadError> {
    need(bytes, at, 4)?;
    let mut b = [0u8; 4];
    b.copy_from_slice(&bytes[at..at + 4]);
    Ok(u32::from_le_bytes(b))
}

fn u64_at(bytes: &[u8], at: usize) -> Result<u64, ReadError> {
    need(bytes, at, 8)?;
    let mut b = [0u8; 8];
    b.copy_from_slice(&bytes[at..at + 8]);
    Ok(u64::from_le_bytes(b))
}

fn f32_at(bytes: &[u8], at: usize) -> Result<f32, ReadError> {
    Ok(f32::from_bits(u32_at(bytes, at)?))
}

fn put_u16(out: &mut [u8], at: usize, value: u16) {
    out[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], at: usize, value: u64) {
    out[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn put_f32(out: &mut [u8], at: usize, value: f32) {
    put_u32(out, at, value.to_bits());
}

/// Liest den Header. Prueft nur Laenge und Formatkennung, keine Semantik.
pub fn parse_header(bytes: &[u8]) -> Result<Header, ReadError> {
    if bytes.len() < HEADER_LEN {
        return Err(ReadError::Truncated { at: bytes.len() });
    }
    if bytes[OFF_MAGIC..OFF_MAGIC + 4] != MAGIC {
        return Err(ReadError::BadMagic);
    }
    Ok(Header {
        format_version: u16_at(bytes, OFF_VERSION)?,
        flags: u16_at(bytes, OFF_FLAGS)?,
        d_model: u32_at(bytes, OFF_D_MODEL)?,
        n_layers: u32_at(bytes, OFF_N_LAYERS)?,
        n_heads: u32_at(bytes, OFF_N_HEADS)?,
        n_kv_heads: u32_at(bytes, OFF_N_KV_HEADS)?,
        d_head: u32_at(bytes, OFF_D_HEAD)?,
        d_ff: u32_at(bytes, OFF_D_FF)?,
        vocab: u32_at(bytes, OFF_VOCAB)?,
        max_seq: u32_at(bytes, OFF_MAX_SEQ)?,
        tensor_count: u32_at(bytes, OFF_TENSOR_COUNT)?,
        dtype: u32_at(bytes, OFF_DTYPE)?,
        layout: u32_at(bytes, OFF_LAYOUT)?,
        reserved: u32_at(bytes, OFF_RESERVED)?,
        tensor_data_bytes: u64_at(bytes, OFF_DATA_BYTES)?,
    })
}

/// Schreibt einen Namen in ein festes Feld. Liefert die Laenge oder `None`.
fn name_into(buf: &mut [u8; 32], parts: &[&str]) -> Option<usize> {
    let mut at = 0;
    for part in parts {
        let bytes = part.as_bytes();
        if at + bytes.len() > 32 {
            return None;
        }
        buf[at..at + bytes.len()].copy_from_slice(bytes);
        at += bytes.len();
    }
    Some(at)
}

/// Erwarteter Name an Position `index` der kanonischen Folge.
///
/// Die Funktion allokiert nichts (fester Puffer) und formatiert den Blockindex
/// selbst, damit der Leser je Tabelleneintrag nichts anlegen muss
/// (`ATC-AI-LLM-002` Abschnitt 8.1, Regel 3).
pub fn expected_name(
    index: usize,
    tied: bool,
    n_layers: usize,
    buf: &mut [u8; 32],
) -> Option<usize> {
    let block_start = if tied { 1 } else { 2 };
    let block_end = block_start + 8 * n_layers;
    if index == 0 {
        return name_into(buf, &["tok_emb"]);
    }
    if !tied && index == 1 {
        return name_into(buf, &["lm_head"]);
    }
    if index >= block_start && index < block_end {
        let rel = index - block_start;
        let block = rel / 8;
        let suffix = BLOCK_SUFFIXES[rel % 8];
        let mut digits = [0u8; 20];
        let n = put_decimal(&mut digits, block);
        let digits = core::str::from_utf8(&digits[..n]).ok()?;
        return name_into(buf, &["block.", digits, ".", suffix]);
    }
    if index == block_end {
        return name_into(buf, &["final_norm"]);
    }
    None
}

/// Dezimaldarstellung ohne fuehrende Nullen in einen Puffer.
fn put_decimal(buf: &mut [u8; 20], mut value: usize) -> usize {
    if value == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 20];
    let mut n = 0;
    while value > 0 {
        tmp[n] = b'0' + (value % 10) as u8;
        value /= 10;
        n += 1;
    }
    for i in 0..n {
        buf[i] = tmp[n - 1 - i];
    }
    n
}

/// Prueft, dass ein Feld nur `[a-z0-9_.]` enthaelt und NUL-aufgefuellt ist.
fn check_name_field(field: &[u8; 32], len: usize) -> bool {
    if len == 0 || len > 32 {
        return false;
    }
    for (i, byte) in field.iter().enumerate() {
        if i < len {
            let ok = byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || *byte == b'_'
                || *byte == b'.';
            if !ok {
                return false;
            }
        } else if *byte != 0 {
            return false;
        }
    }
    true
}

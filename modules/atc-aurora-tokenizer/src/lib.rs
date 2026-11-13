// atc-aurora-tokenizer — nativer Byte-Level-BPE-Tokenizer, M-04
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Eigentuemer der Textzerlegung. Keine Abhaengigkeit auf `tokenizers`, `tiktoken`
// oder eine andere Fremdbibliothek (ATC-AI-LLM-001 §3.2): Vokabular, Merge-Tabelle
// und Digest entstehen im eigenen Code. Das Crate baut ohne std.
//
// Zwei Eigenschaften sind bindend und werden in `tests.rs` geprueft:
//   1. Verlustfreiheit: decode(encode(b)) == b fuer beliebige Bytefolgen, auch
//      fuer ungueltiges UTF-8 (B-14).
//   2. Determinismus: zwei Trainingslaeufe auf identischer Quelle liefern
//      identisches Vokabular und identischen Digest (B-15). Ohne festen
//      Tie-Break ist BPE-Training nicht reproduzierbar.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod bpe;
pub mod error;
pub mod sha256;

#[cfg(test)]
mod tests;

pub use bpe::{Bpe, BASE_VOCAB_SIZE, CANONICAL_MAGIC};
pub use error::Error;

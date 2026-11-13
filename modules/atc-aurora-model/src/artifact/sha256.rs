//! SHA-256-Fassade des Modells — seit AD-FT1 nur noch ein Re-Export.
//!
//! Vor AD-FT1 stand hier eine eigene Umsetzung. Sie war bytegenau gleich zur
//! Tokenizer-Fassung (geprueft ueber 301 Laengen plus Padding-Grenzfaelle), aber
//! sie war eine zweite Stelle, an der ein Krypto-Algorithmus haette abdriften
//! koennen. Die Autoritaet ist `atc-aurora-digest` (Streaming-API), dieses Modul
//! haelt nur die Namen stabil, die `read`, `write` und die Tests benutzen.

pub use atc_aurora_digest::{digest, Sha256};

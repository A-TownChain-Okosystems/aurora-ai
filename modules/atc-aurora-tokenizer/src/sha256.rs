//! Digest-Fassade des Tokenizers — seit AD-FT1 nur noch ein Re-Export.
//!
//! Befund F-T1 war: zwei Umsetzungen von SHA-256 im Repo, bytegenau gleich, aber
//! doppelt gepflegt. AD-FT1 entscheidet die Zusammenfuehrung ueber ein gemeinsames
//! Crate, nicht durch Unterordnung des Tokenizers unter das Model. Die Autoritaet
//! ist `atc-aurora-digest` mit der Streaming-API; dieses Modul haelt die Namen
//! stabil, die `bpe` und die Tests benutzen, und baut keine Eingabekopie.

pub use atc_aurora_digest::{hex, sha256, sha256_hex};

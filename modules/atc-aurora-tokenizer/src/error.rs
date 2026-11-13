// Typisierte Fehler des Tokenizers. Kein Panik, kein stilles Ersatzzeichen:
// jede Verletzung endet als `Err` (Blueprint B-16, B-17).

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Zielvokabular kleiner als die 256 Byte-Token.
    TargetVocabTooSmall(usize),
    /// Zielvokabular oberhalb der harten Grenze (65536).
    TargetVocabTooLarge(usize),
    /// Ziel beim Training nicht erreicht, weil keine Paarung mehr zweimal auftritt.
    TargetUnreachable { reached: usize, target: usize },
    /// Kanonische Form verletzt die Struktur.
    Malformed(&'static str),
    /// Der Digest der geladenen Bytes passt nicht zum erwarteten.
    DigestMismatch,
    /// Token-ID ausserhalb des Vokabulars.
    UnknownToken(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TargetVocabTooSmall(v) => {
                write!(f, "Zielvokabular {v} liegt unter den 256 Byte-Token")
            }
            Error::TargetVocabTooLarge(v) => {
                write!(f, "Zielvokabular {v} liegt ueber der Grenze 65536")
            }
            Error::TargetUnreachable { reached, target } => write!(
                f,
                "Ziel {target} nicht erreichbar: keine Paarung mehr zweimal vorhanden, erreicht {reached}"
            ),
            Error::Malformed(was) => write!(f, "kanonische Form verletzt: {was}"),
            Error::DigestMismatch => write!(
                f,
                "Digest-Mismatch: die geladenen Bytes sind nicht die Bytes, deren Digest erwartet wurde"
            ),
            Error::UnknownToken(id) => write!(f, "Token {id} ist im Vokabular nicht vorhanden"),
        }
    }
}

impl core::error::Error for Error {}

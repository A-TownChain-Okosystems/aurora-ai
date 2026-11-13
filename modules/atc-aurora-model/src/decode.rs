//! Greedy-Dekodierung (P4): die Inferenzseite der Modellkette.
//!
//! Bewusst klein und ohne Zustand: je Schritt ein vollstaendiger Vorwaertslauf
//! ueber das bisherige Praefix, dann das wahrscheinlichste Token. Das ist
//! **nicht** die spaetere Laufzeit — KV-Cache, Batching und Sampling gehoeren
//! zu P8/`atc-aurora-runtime` —, sondern der Nachweis, dass der inferenzfaehige
//! Pfad aus demselben `forward` besteht, den die Tests bereits abdecken. Wer
//! hier eine zweite Rechnung einbaut, baut einen zweiten Vorwaertslauf.
//!
//! Die Auswahl ist `argmax` mit dem kleinsten Index bei Gleichstand. Damit ist
//! die Ausgabe fuer dieselben Gewichte und dieselbe Eingabe reproduzierbar und
//! ohne Zufallsstrom pruefbar.

use crate::config::Config;
use crate::forward::{forward, ForwardError};
use crate::weights::Weights;
use atc_aurora_tensor::Budget;

/// Ergebnis einer Dekodierung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// Praefix plus erzeugte Token.
    pub tokens: Vec<u32>,
    /// Anzahl der tatsaechlich erzeugten Token.
    pub generated: usize,
}

/// Erzeugt hoechstens `max_new` Token nach `prompt`.
///
/// Grenzen: das Praefix darf nicht leer sein und nicht laenger als
/// `cfg.max_seq`; ist es bereits voll, werden keine Token erzeugt und das
/// Praefix kommt unveraendert zurueck. `stop` beendet die Erzeugung, sobald
/// dieses Token entsteht (das Stopp-Token ist enthalten).
///
/// Das uebergebene Budget wird je Schritt zurueckgesetzt, damit der Aufrufer
/// keine Summe ueber alle Schritte vorhalten muss.
pub fn generate(
    cfg: &Config,
    w: &Weights,
    prompt: &[u32],
    max_new: usize,
    stop: Option<u32>,
    budget: &mut Budget,
) -> Result<Decoded, ForwardError> {
    if prompt.is_empty() {
        return Err(ForwardError::EmptySequence);
    }
    if prompt.len() > cfg.max_seq {
        return Err(ForwardError::SequenceTooLong {
            len: prompt.len(),
            max: cfg.max_seq,
        });
    }

    let mut tokens = prompt.to_vec();
    let mut generated = 0usize;
    for _ in 0..max_new {
        if tokens.len() >= cfg.max_seq {
            break;
        }
        budget.clear();
        let logits = forward(cfg, w, &tokens, budget)?;
        let t = tokens.len();
        let v = cfg.vocab;
        let last = &logits.as_slice()[(t - 1) * v..t * v];
        let mut best = 0usize;
        for (i, value) in last.iter().enumerate() {
            // Strikte Groesser-Prüfung: bei Gleichstand gewinnt der kleinere
            // Index, damit die Auswahl deterministisch ist.
            if *value > last[best] {
                best = i;
            }
        }
        let next = best as u32;
        tokens.push(next);
        generated += 1;
        if stop == Some(next) {
            break;
        }
    }
    Ok(Decoded { tokens, generated })
}

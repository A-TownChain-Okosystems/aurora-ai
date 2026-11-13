// atc-aurora-train — Trainings-Engine der Aurora-Referenz (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Diese Crate schliesst die Modellkette:
//
//   Tokenizer -> Autograd -> Backpropagation -> Optimizer -> Training
//   -> Evaluation -> Registry -> Inferenz
//
// Sie besitzt nur den Teil ab dem Optimierer. Autograd und Rueckwaertspass
// stehen in `atc-aurora-tensor` (M-03), der differenzierbare und der
// inferenzfaehige Vorwaertslauf in `atc-aurora-model` (M-05), die Pruefsumme in
// `atc-aurora-digest` (AD-FT1). Es gibt hier **keine** zweite Tensor-, Norm-,
// Attention- oder Hash-Implementierung; ein Test prueft, dass die Gewichte
// eines Pruefpunkts bytegleich zurueckkommen.
//
// Kein Zufall, kein Betriebssystemzustand, kein Dateizugriff: derselbe Korpus
// mit denselben Gewichten ergibt auf jeder Maschine denselben Verlauf.

#![forbid(unsafe_code)]

pub mod error;
pub mod eval;
pub mod gradients;
pub mod optim;
pub mod registry;
pub mod state;
pub mod trainer;

pub use error::TrainError;
pub use eval::{evaluate, EvalReport};
pub use gradients::Gradients;
pub use optim::{Adam, HyperParams};
pub use registry::{checkpoint_from, restore, Checkpoint, Registry};
pub use state::TrainerState;
pub use trainer::{StepReport, Trainer};

#[cfg(test)]
mod tests;

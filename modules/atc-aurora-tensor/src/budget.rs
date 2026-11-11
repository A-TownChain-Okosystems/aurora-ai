// atc-aurora-tensor — Speicherbudget
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Vertrag nach B-03: die Budgetprüfung findet VOR der Zuteilung statt. Wer
// `reserve` nicht erfolgreich aufgerufen hat, bekommt keinen Puffer. Der Name
// `BudgetExceeded` ist an `ResourceError` aus der kai-os-Laufzeit angelehnt,
// damit im Ökosystem nicht zwei Budgetsprachen entstehen.

use crate::error::{Result, TensorError};

/// Verwaltetes Bytebudget für Tensorpuffer.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    limit: usize,
    used: usize,
}

impl Budget {
    /// `limit_bytes` ist die Obergrenze für gleichzeitig belegte Tensorpuffer.
    pub const fn new(limit_bytes: usize) -> Self {
        Budget {
            limit: limit_bytes,
            used: 0,
        }
    }

    /// Unbegrenztes Budget für Aufrufer, die ihre Grenze selbst führen.
    pub const fn unlimited() -> Self {
        Budget {
            limit: usize::MAX,
            used: 0,
        }
    }

    pub const fn limit(&self) -> usize {
        self.limit
    }

    pub const fn used(&self) -> usize {
        self.used
    }

    pub const fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.used)
    }

    /// Prüft und verbucht `bytes`. Bei Überschreitung bleibt der Zustand
    /// unverändert und es wurde nichts zugeteilt.
    pub fn reserve(&mut self, bytes: usize) -> Result<()> {
        let next = self
            .used
            .checked_add(bytes)
            .ok_or(TensorError::SizeOverflow)?;
        if next > self.limit {
            return Err(TensorError::BudgetExceeded {
                requested: bytes,
                limit: self.limit,
            });
        }
        self.used = next;
        Ok(())
    }

    /// Gibt zuvor reservierte Bytes frei.
    pub fn release(&mut self, bytes: usize) {
        self.used = self.used.saturating_sub(bytes);
    }

    /// Setzt das Budget auf null Belegung zurück.
    pub fn clear(&mut self) {
        self.used = 0;
    }
}

impl Default for Budget {
    fn default() -> Self {
        Budget::unlimited()
    }
}

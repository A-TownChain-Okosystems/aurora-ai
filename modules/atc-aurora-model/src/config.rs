//! Konfiguration des Referenzmodells.

/// Grund, aus dem eine Konfiguration nicht zulaessig ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// Eine der Groessen ist null.
    ZeroDimension,
    /// `n_heads * d_head` passt nicht zu `d_model`.
    HeadDimMismatch,
    /// `n_kv_heads` ist null oder groesser als `n_heads` oder teilt `n_heads` nicht.
    KvHeadMismatch,
}

/// Architektur der Referenz. Alle Felder sind Anzahl von Elementen, nicht Bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Modellbreite.
    pub d_model: usize,
    /// Anzahl der Transformerbloecke.
    pub n_layers: usize,
    /// Anzahl der Abfragekoepfe.
    pub n_heads: usize,
    /// Anzahl der Schluessel-/Wertkoepfe (gleich `n_heads` bedeutet MHA).
    pub n_kv_heads: usize,
    /// Breite eines Kopfes.
    pub d_head: usize,
    /// Breite des Zwischenraums im SwiGLU-Block.
    pub d_ff: usize,
    /// Vokabulargroesse.
    pub vocab: usize,
    /// Maximale Sequenzlaenge.
    pub max_seq: usize,
    /// Einbettung und Ausgabeprojektion teilen sich die Gewichte.
    pub tied_embedding: bool,
}

impl Config {
    /// Die eingefrorene Referenz (K4: Vokabular 4096, K5: d_ff 704).
    pub const fn ref1() -> Self {
        Self {
            d_model: 256,
            n_layers: 6,
            n_heads: 8,
            n_kv_heads: 8,
            d_head: 32,
            d_ff: 704,
            vocab: 4096,
            max_seq: 512,
            tied_embedding: true,
        }
    }

    /// Prueft die Konfiguration gegen die Architekturbedingungen.
    pub fn validate(&self) -> Result<(), RefusalReason> {
        let dims = [
            self.d_model,
            self.n_layers,
            self.n_heads,
            self.n_kv_heads,
            self.d_head,
            self.d_ff,
            self.vocab,
            self.max_seq,
        ];
        if dims.contains(&0) {
            return Err(RefusalReason::ZeroDimension);
        }
        if self.n_heads * self.d_head != self.d_model {
            return Err(RefusalReason::HeadDimMismatch);
        }
        if self.n_kv_heads > self.n_heads || !self.n_heads.is_multiple_of(self.n_kv_heads) {
            return Err(RefusalReason::KvHeadMismatch);
        }
        Ok(())
    }
}

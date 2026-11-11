//! Gewichte der Referenz und ihre deterministische Initialisierung.
//!
//! Die Formen folgen `params.rs`: die Summe aller Elemente muss genau die dort
//! gezaehlte Parameterzahl ergeben. Diese Invariante ist der Grund, warum hier
//! kein Tensor "frei" entsteht — jede Zuteilung stammt aus der Konfiguration.

use crate::config::Config;
use atc_aurora_tensor::Tensor;

/// Ein Transformerblock: zwei Normen, vier Attention-Matrizen, drei MLP-Matrizen.
#[derive(Debug, Clone)]
pub struct BlockWeights {
    /// RMSNorm vor der Attention, Breite `d_model`.
    pub attn_norm: Tensor,
    /// Abfrageprojektion `(d_model, n_heads * d_head)`.
    pub wq: Tensor,
    /// Schluesselprojektion `(d_model, n_kv_heads * d_head)`.
    pub wk: Tensor,
    /// Wertprojektion `(d_model, n_kv_heads * d_head)`.
    pub wv: Tensor,
    /// Ausgabeprojektion `(n_heads * d_head, d_model)`.
    pub wo: Tensor,
    /// RMSNorm vor dem MLP, Breite `d_model`.
    pub mlp_norm: Tensor,
    /// Gate und Up gemeinsam `(d_model, 2 * d_ff)`; erste Haelfte Gate.
    pub wgu: Tensor,
    /// Down-Projektion `(d_ff, d_model)`.
    pub wd: Tensor,
}

/// Alle Gewichte der Referenz. Keine Bias-Terme (ATC-AI-LLM-001 Abschnitt 5.1).
#[derive(Debug, Clone)]
pub struct Weights {
    /// Einbettungstabelle `(vocab, d_model)`, zugleich Ausgabekopf (gebunden).
    pub tok_emb: Tensor,
    /// Ein Block je Schicht.
    pub blocks: Vec<BlockWeights>,
    /// Abschliessende RMSNorm vor dem Ausgabekopf.
    pub final_norm: Tensor,
}

/// Einfacher, plattformunabhaengiger Zufallsstrom (splitmix64).
///
/// Bewusst kein `rand`: die Initialisierung muss auf jedem Rechner dieselben
/// Werte ergeben, ohne Abhaengigkeit und ohne Betriebssystemzufall.
#[derive(Debug, Clone, Copy)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// Neuer Strom aus einem Startwert.
    pub const fn new(seed: u64) -> Self {
        SplitMix64(seed)
    }

    /// Naechste 64 Bit.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Gleichverteilt in `[-1, 1)`.
    pub fn next_signed(&mut self) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        unit * 2.0 - 1.0
    }
}

fn tensor(shape: Vec<usize>, rng: &mut SplitMix64, scale: f32) -> Tensor {
    let n: usize = shape.iter().product();
    let mut data = Vec::with_capacity(n);
    for _ in 0..n {
        data.push(rng.next_signed() * scale);
    }
    Tensor::new(shape, data).expect("Form und Datenlaenge stammen aus derselben Rechnung")
}

impl BlockWeights {
    fn new(cfg: &Config, rng: &mut SplitMix64, scale: f32) -> Self {
        let d = cfg.d_model;
        let q = cfg.n_heads * cfg.d_head;
        let kv = cfg.n_kv_heads * cfg.d_head;
        BlockWeights {
            attn_norm: Tensor::from_vec(vec![1.0; d]),
            wq: tensor(vec![d, q], rng, scale),
            wk: tensor(vec![d, kv], rng, scale),
            wv: tensor(vec![d, kv], rng, scale),
            wo: tensor(vec![q, d], rng, scale),
            mlp_norm: Tensor::from_vec(vec![1.0; d]),
            wgu: tensor(vec![d, 2 * cfg.d_ff], rng, scale),
            wd: tensor(vec![cfg.d_ff, d], rng, scale),
        }
    }
}

impl Weights {
    /// Gewichte fuer eine Konfiguration. `scale` ist die Streuung der
    /// Projektionsmatrizen; Normgewichte starten bei 1, damit die Norm ein
    /// Neutralverhalten hat und nicht schon durch die Initialisierung daempft.
    pub fn deterministic(cfg: &Config, seed: u64, scale: f32) -> Self {
        let mut rng = SplitMix64::new(seed);
        let blocks = (0..cfg.n_layers)
            .map(|_| BlockWeights::new(cfg, &mut rng, scale))
            .collect();
        Weights {
            tok_emb: tensor(vec![cfg.vocab, cfg.d_model], &mut rng, scale),
            blocks,
            final_norm: Tensor::from_vec(vec![1.0; cfg.d_model]),
        }
    }

    /// Gewichte mit der ueblichen Skalierung `1/sqrt(d_model)`.
    pub fn from_seed(cfg: &Config, seed: u64) -> Self {
        let scale = 1.0 / (cfg.d_model as f32).sqrt();
        Self::deterministic(cfg, seed, scale)
    }

    /// Summe aller Gewichtselemente. Muss `param_count(cfg)` entsprechen.
    pub fn param_count(&self) -> usize {
        let mut n = self.tok_emb.size() + self.final_norm.size();
        for b in &self.blocks {
            n += b.attn_norm.size()
                + b.wq.size()
                + b.wk.size()
                + b.wv.size()
                + b.wo.size()
                + b.mlp_norm.size()
                + b.wgu.size()
                + b.wd.size();
        }
        n
    }
}

//! Vorwaertslauf der Referenz (M-05).
//!
//! Ein Token je Zeile, eine Zeile je Position: `(T, d_model)` durch sechs
//! Bloecke, abschliessende Norm, gebundener Ausgabekopf. Pre-Norm, kein Bias,
//! SwiGLU, RoPE, kausale Attention mit `j <= i`.
//!
//! Die Rechenoperationen kommen aus `atc-aurora-tensor` (M-01 bis M-03); hier
//! steht nur die Verdrahtung. Keine Operation ist zweitimplementiert, damit es
//! genau eine Stelle je Formel gibt.

use crate::config::Config;
use crate::weights::Weights;
use atc_aurora_tensor::{Budget, Tensor, TensorError};

/// Epsilon der RMSNorm (B-11).
pub const NORM_EPS: f32 = 1e-8;

/// Basis der Rotationsfrequenzen. Noch nicht normativ festgelegt, siehe STATUS.
pub const ROPE_BASE: f32 = 10_000.0;

/// Fehler des Vorwaertslaufs. Kein `String`, damit ein Fehler keine Zuteilung
/// ausloest; dieselbe Regel wie im Tensor-Kern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForwardError {
    /// Eine Tensoroperation hat abgelehnt.
    Tensor(TensorError),
    /// Die Eingabe ist leer.
    EmptySequence,
    /// Die Eingabe ist laenger als der Kontext der Konfiguration.
    SequenceTooLong { len: usize, max: usize },
    /// Ein Token liegt ausserhalb des Vokabulars.
    TokenOutOfRange { token: u32, vocab: usize },
    /// Die Gewichte passen nicht zu dieser Konfiguration.
    WeightMismatch { what: &'static str },
}

impl From<TensorError> for ForwardError {
    fn from(e: TensorError) -> Self {
        ForwardError::Tensor(e)
    }
}

/// Vorberechnete Rotationsfrequenzen und ihre Sinus-/Kosinustafel.
struct Rope {
    cos: Vec<f32>,
    sin: Vec<f32>,
    half: usize,
}

impl Rope {
    fn new(d_head: usize, max_positions: usize) -> Self {
        let half = d_head / 2;
        let mut cos = vec![0.0f32; max_positions * half];
        let mut sin = vec![0.0f32; max_positions * half];
        for i in 0..half {
            // theta_i = base^(-2i/d_head)
            let theta = ROPE_BASE.powf(-(2.0 * i as f32) / d_head as f32);
            for pos in 0..max_positions {
                let angle = pos as f32 * theta;
                cos[pos * half + i] = angle.cos();
                sin[pos * half + i] = angle.sin();
            }
        }
        Rope { cos, sin, half }
    }

    /// Dreht `q` an Ort und Stelle, je Position und Kopf, paarweise `(2i, 2i+1)`.
    fn apply(
        &self,
        t: Tensor,
        rows: usize,
        heads: usize,
        d_head: usize,
    ) -> Result<Tensor, ForwardError> {
        let shape = t.shape().to_vec();
        let mut data = t.into_vec();
        let half = self.half;
        for pos in 0..rows {
            for h in 0..heads {
                let base = (pos * heads + h) * d_head;
                for i in 0..half {
                    let a = data[base + 2 * i];
                    let b = data[base + 2 * i + 1];
                    let c = self.cos[pos * half + i];
                    let s = self.sin[pos * half + i];
                    data[base + 2 * i] = a * c - b * s;
                    data[base + 2 * i + 1] = a * s + b * c;
                }
            }
        }
        Tensor::new(shape, data).map_err(ForwardError::Tensor)
    }
}

fn check_weights(cfg: &Config, w: &Weights) -> Result<(), ForwardError> {
    let d = cfg.d_model;
    let q = cfg.n_heads * cfg.d_head;
    let kv = cfg.n_kv_heads * cfg.d_head;
    if w.tok_emb.shape() != [cfg.vocab, d] {
        return Err(ForwardError::WeightMismatch { what: "tok_emb" });
    }
    if w.final_norm.shape() != [d] {
        return Err(ForwardError::WeightMismatch { what: "final_norm" });
    }
    if w.blocks.len() != cfg.n_layers {
        return Err(ForwardError::WeightMismatch { what: "blocks" });
    }
    for b in &w.blocks {
        let ok = b.attn_norm.shape() == [d]
            && b.mlp_norm.shape() == [d]
            && b.wq.shape() == [d, q]
            && b.wk.shape() == [d, kv]
            && b.wv.shape() == [d, kv]
            && b.wo.shape() == [q, d]
            && b.wgu.shape() == [d, 2 * cfg.d_ff]
            && b.wd.shape() == [cfg.d_ff, d];
        if !ok {
            return Err(ForwardError::WeightMismatch { what: "block" });
        }
    }
    Ok(())
}

/// Kausale Multi-Head-Attention ohne Bias.
///
/// `q` ist `(T, n_heads*d_head)`, `k` und `v` sind `(T, n_kv_heads*d_head)`.
/// Jede Abfrageposition `i` sieht nur Schluessel `j <= i`; alles darueber wird
/// vor der Softmax auf `-inf` gesetzt, damit ein Maskenfehler nicht als kleiner
/// Zahlenwert durchgeht, sondern die Zeile garantiert nicht beeinflusst.
pub(crate) fn attention(
    cfg: &Config,
    q: &Tensor,
    k: &Tensor,
    v: &Tensor,
    budget: &mut Budget,
) -> Result<Tensor, ForwardError> {
    let t = q.shape()[0];
    let d_head = cfg.d_head;
    let n_heads = cfg.n_heads;
    let groups = cfg.n_heads / cfg.n_kv_heads;
    let q_dim = n_heads * d_head;
    let kv_dim = cfg.n_kv_heads * d_head;
    let scale = 1.0 / (d_head as f32).sqrt();

    let qd = q.as_slice();
    let kd = k.as_slice();
    let vd = v.as_slice();
    let mut out = vec![0.0f32; t * q_dim];

    for h in 0..n_heads {
        let kvh = h / groups;
        let mut scores = vec![f32::NEG_INFINITY; t * t];
        for i in 0..t {
            for j in 0..=i {
                let mut acc = 0.0f32;
                for d in 0..d_head {
                    acc += qd[i * q_dim + h * d_head + d] * kd[j * kv_dim + kvh * d_head + d];
                }
                scores[i * t + j] = acc * scale;
            }
        }
        let probs = Tensor::new(vec![t, t], scores)?.softmax(budget)?;
        let mut vh = vec![0.0f32; t * d_head];
        for j in 0..t {
            for d in 0..d_head {
                vh[j * d_head + d] = vd[j * kv_dim + kvh * d_head + d];
            }
        }
        let v_tensor = Tensor::new(vec![t, d_head], vh)?;
        let head_out = probs.matmul(&v_tensor, budget)?;
        let data = head_out.as_slice();
        for i in 0..t {
            for d in 0..d_head {
                out[i * q_dim + h * d_head + d] = data[i * d_head + d];
            }
        }
    }

    Tensor::new(vec![t, q_dim], out).map_err(ForwardError::Tensor)
}

/// Vorwaertslauf: Tokens hinein, Logits `(T, vocab)` heraus.
pub fn forward(
    cfg: &Config,
    w: &Weights,
    tokens: &[u32],
    budget: &mut Budget,
) -> Result<Tensor, ForwardError> {
    cfg.validate()
        .map_err(|_| ForwardError::WeightMismatch { what: "config" })?;
    check_weights(cfg, w)?;
    let t = tokens.len();
    if t == 0 {
        return Err(ForwardError::EmptySequence);
    }
    if t > cfg.max_seq {
        return Err(ForwardError::SequenceTooLong {
            len: t,
            max: cfg.max_seq,
        });
    }
    let d = cfg.d_model;

    // Einbettung: Zeilen ziehen, nicht multiplizieren.
    let emb_table = w.tok_emb.as_slice();
    let mut emb = Vec::with_capacity(t * d);
    for &tok in tokens {
        let idx = tok as usize;
        if idx >= cfg.vocab {
            return Err(ForwardError::TokenOutOfRange {
                token: tok,
                vocab: cfg.vocab,
            });
        }
        emb.extend_from_slice(&emb_table[idx * d..(idx + 1) * d]);
    }
    let mut x = Tensor::new(vec![t, d], emb)?;

    let rope = Rope::new(cfg.d_head, t);
    for b in &w.blocks {
        let xn = x.rms_norm(&b.attn_norm, NORM_EPS, budget)?;
        let q = xn.matmul(&b.wq, budget)?;
        let k = xn.matmul(&b.wk, budget)?;
        let v = xn.matmul(&b.wv, budget)?;
        let q = rope.apply(q, t, cfg.n_heads, cfg.d_head)?;
        let k = rope.apply(k, t, cfg.n_kv_heads, cfg.d_head)?;
        let ctx = attention(cfg, &q, &k, &v, budget)?;
        let attn_out = ctx.matmul(&b.wo, budget)?;
        x = x.add(&attn_out, budget)?;

        let xn2 = x.rms_norm(&b.mlp_norm, NORM_EPS, budget)?;
        let gate_up = xn2.matmul(&b.wgu, budget)?;
        let act = gate_up.swiglu(budget)?;
        let down = act.matmul(&b.wd, budget)?;
        x = x.add(&down, budget)?;
    }

    let x = x.rms_norm(&w.final_norm, NORM_EPS, budget)?;
    // Gebundene Einbettung: der Ausgabekopf ist die transponierte Tabelle.
    let head = w.tok_emb.transpose(budget)?;
    let logits = x.matmul(&head, budget)?;
    Ok(logits)
}

//! Differenzierbarer Vorwaertslauf (P4): derselbe Rechenweg wie `forward`, aber
//! als Graph aus `atc-aurora-tensor` (M-03), damit der Rueckwaertspass ihn
//! ableiten kann.
//!
//! Warum das kein zweiter Vorwaertslauf ist: die Formeln stehen weiterhin genau
//! einmal, naemlich in den Operationen von `atc-aurora-tensor`. Hier steht die
//! Verdrahtung, und die ist gegen `forward` geprueft (Test
//! `p4_graph_logits_stimmen_mit_forward_ueberein`): derselbe Baum, dieselben
//! Gewichte, dieselben Positionen, Abweichung nur aus der Summationsreihenfolge
//! der kopfweisen Ausgabeprojektion.
//!
//! Die Parameter sind Blattknoten in der kanonischen Tensorfolge von
//! [`Weights::tensors`]. Damit ist der Index eines Gradienten derselbe Index wie
//! der des Parameters; der Optimierer (M-06) braucht keine Namensaufloesung.

use crate::config::Config;
use crate::forward::{ForwardError, NORM_EPS, ROPE_BASE};
use crate::weights::Weights;
use atc_aurora_tensor::{Budget, Graph, NodeId, Tensor};

/// Ergebnis des Graphen: der Rechengraph, der Logits-Knoten und die
/// Parameterknoten in kanonischer Folge.
pub struct GraphForward {
    /// Der Rechengraph. Er besitzt Werte und Gradienten; `clear` gibt alles frei.
    pub graph: Graph,
    /// Logits `(T, vocab)`.
    pub logits: NodeId,
    /// Parameterknoten in der Reihenfolge von [`Weights::tensors`].
    pub params: Vec<NodeId>,
}

/// Knoten eines Blocks, in der Reihenfolge der kanonischen Tensorfolge.
struct BlockIds {
    attn_norm: NodeId,
    wq: NodeId,
    wk: NodeId,
    wv: NodeId,
    wo: NodeId,
    mlp_norm: NodeId,
    wgu: NodeId,
    wd: NodeId,
}

/// Rotationskonstanten als Blattknoten, je Position und Paar.
///
/// Dieselbe Formel wie `Rope::new` im Vorwaertslauf: `theta_i = base^(-2i/d_head)`.
fn rope_tables(graph: &mut Graph, t: usize, d_head: usize) -> (NodeId, NodeId) {
    let half = d_head / 2;
    let mut cos = vec![0.0f32; t * half];
    let mut sin = vec![0.0f32; t * half];
    for i in 0..half {
        let theta = ROPE_BASE.powf(-(2.0 * i as f32) / d_head as f32);
        for pos in 0..t {
            let angle = pos as f32 * theta;
            cos[pos * half + i] = angle.cos();
            sin[pos * half + i] = angle.sin();
        }
    }
    let c =
        graph.leaf(Tensor::new(vec![t, half], cos).expect("Form und Daten aus derselben Rechnung"));
    let s =
        graph.leaf(Tensor::new(vec![t, half], sin).expect("Form und Daten aus derselben Rechnung"));
    (c, s)
}

/// Kausalmaske als Konstante: `0` auf und unter der Diagonale, `-inf` darueber.
fn causal_mask(graph: &mut Graph, t: usize) -> NodeId {
    let mut m = vec![0.0f32; t * t];
    for i in 0..t {
        for j in (i + 1)..t {
            m[i * t + j] = f32::NEG_INFINITY;
        }
    }
    graph.leaf(Tensor::new(vec![t, t], m).expect("Form und Daten aus derselben Rechnung"))
}

/// Vorwaertslauf als Graph: Tokens hinein, Logits-Knoten und Parameterknoten
/// heraus.
///
/// Die Grenzen sind dieselben wie in [`crate::forward::forward`]; ein
/// unzulaessiger Aufruf wird typisiert abgelehnt.
pub fn forward_graph(
    cfg: &Config,
    w: &Weights,
    tokens: &[u32],
    budget: &mut Budget,
) -> Result<GraphForward, ForwardError> {
    cfg.validate()
        .map_err(|_| ForwardError::WeightMismatch { what: "config" })?;
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
    for &tok in tokens {
        if tok as usize >= cfg.vocab {
            return Err(ForwardError::TokenOutOfRange {
                token: tok,
                vocab: cfg.vocab,
            });
        }
    }

    let mut graph = Graph::new();
    let tensors = w.tensors();
    let mut params: Vec<NodeId> = Vec::with_capacity(tensors.len());
    for tensor in tensors {
        params.push(graph.parameter(tensor.clone()));
    }

    // Bindung der Rollen an die kanonische Folge. Die Zahl der Tensoren wird
    // geprueft, damit eine Konfiguration ohne passende Gewichte nicht in eine
    // falsche Zuordnung laeuft.
    let mut k = 0usize;
    let tok_emb = *params.first().ok_or(ForwardError::WeightMismatch {
        what: "tensor_count",
    })?;
    k += 1;
    let lm_head = if cfg.tied_embedding {
        None
    } else {
        let id = *params.get(k).ok_or(ForwardError::WeightMismatch {
            what: "tensor_count",
        })?;
        k += 1;
        Some(id)
    };
    let mut blocks: Vec<BlockIds> = Vec::with_capacity(cfg.n_layers);
    for _ in 0..cfg.n_layers {
        if k + 8 > params.len() {
            return Err(ForwardError::WeightMismatch {
                what: "tensor_count",
            });
        }
        blocks.push(BlockIds {
            attn_norm: params[k],
            wq: params[k + 1],
            wk: params[k + 2],
            wv: params[k + 3],
            wo: params[k + 4],
            mlp_norm: params[k + 5],
            wgu: params[k + 6],
            wd: params[k + 7],
        });
        k += 8;
    }
    let final_norm = *params.get(k).ok_or(ForwardError::WeightMismatch {
        what: "tensor_count",
    })?;
    k += 1;
    if k != params.len() {
        return Err(ForwardError::WeightMismatch {
            what: "tensor_count",
        });
    }

    let (cos, sin) = rope_tables(&mut graph, t, cfg.d_head);
    let mask = causal_mask(&mut graph, t);
    let scale = 1.0 / (cfg.d_head as f32).sqrt();
    let groups = cfg.n_heads / cfg.n_kv_heads;

    let mut x = graph.gather(tok_emb, tokens, budget)?;
    for b in &blocks {
        let xn = graph.rms_norm(x, b.attn_norm, NORM_EPS, budget)?;
        let q = graph.matmul(xn, b.wq, budget)?;
        let kk = graph.matmul(xn, b.wk, budget)?;
        let v = graph.matmul(xn, b.wv, budget)?;
        let q = graph.rope(q, cos, sin, cfg.d_head, budget)?;
        let kk = graph.rope(kk, cos, sin, cfg.d_head, budget)?;

        // Kopfweise Attention. Die Ausgabeprojektion wird je Kopf auf den
        // passenden Zeilenblock von `wo` angewendet und summiert; damit braucht
        // der Graph keine Zusammenfuege-Operation, und `wo` bekommt seinen
        // Gradienten aus denselben Knoten wie im Vorwaertslauf.
        let wo_t = graph.transpose(b.wo, budget)?;
        let mut acc: Option<NodeId> = None;
        for h in 0..cfg.n_heads {
            let kvh = h / groups;
            let qh = graph.slice_cols(q, h * cfg.d_head, cfg.d_head, budget)?;
            let kh = graph.slice_cols(kk, kvh * cfg.d_head, cfg.d_head, budget)?;
            let vh = graph.slice_cols(v, kvh * cfg.d_head, cfg.d_head, budget)?;
            let kt = graph.transpose(kh, budget)?;
            let scores = graph.matmul(qh, kt, budget)?;
            let scores = graph.scale(scores, scale, budget)?;
            let scores = graph.add(scores, mask, budget)?;
            let probs = graph.softmax(scores, budget)?;
            let ctx = graph.matmul(probs, vh, budget)?;
            let woh_t = graph.slice_cols(wo_t, h * cfg.d_head, cfg.d_head, budget)?;
            let woh = graph.transpose(woh_t, budget)?;
            let part = graph.matmul(ctx, woh, budget)?;
            acc = Some(match acc {
                Some(a) => graph.add(a, part, budget)?,
                None => part,
            });
        }
        let attn = acc.ok_or(ForwardError::WeightMismatch { what: "n_heads" })?;
        x = graph.add(x, attn, budget)?;

        let xn2 = graph.rms_norm(x, b.mlp_norm, NORM_EPS, budget)?;
        let gu = graph.matmul(xn2, b.wgu, budget)?;
        let act = graph.swiglu(gu, budget)?;
        let down = graph.matmul(act, b.wd, budget)?;
        x = graph.add(x, down, budget)?;
    }

    let x = graph.rms_norm(x, final_norm, NORM_EPS, budget)?;
    let head = match lm_head {
        Some(id) => id,
        None => graph.transpose(tok_emb, budget)?,
    };
    let logits = graph.matmul(x, head, budget)?;
    Ok(GraphForward {
        graph,
        logits,
        params,
    })
}

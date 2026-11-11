// atc-aurora-tensor — Rechengraph und Rückwärtspass (M-03)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Vertrag: `AUTOGRAD-CONTRACT.md` in diesem Crate. Kurzfassung:
//
//   * Nur die dort aufgeführten Operationen haben eine Ableitung. Es gibt keinen
//     generischen Fallback, der für eine unbekannte Operation "irgendwie"
//     weiterrechnet.
//   * Der Graph besitzt seine Werte. Ein Knoten zeigt auf seine Eingaben über
//     Index, nie über Referenz. Damit kann kein Gradient auf einen Puffer
//     zeigen, den der Aufrufer zwischenzeitlich weggelegt hat.
//   * Jede Zuteilung im Vorwärts- wie im Rückwärtspfad läuft über `Budget` und
//     wird vor der Zuteilung geprüft.
//   * Der Rückwärtspass ist transaktional: schlägt er in der Mitte fehl, wird
//     jede in diesem Aufruf reservierte Byte wieder freigegeben. Ein Fehler
//     hinterlässt keinen halben Gradienten und keine halbe Buchung.
//   * Gradienten werden bei mehrfachem Gebrauch addiert (B-12), nicht
//     überschrieben. Die Reihenfolge der Addition ist die umgekehrte
//     Konstruktionsreihenfolge und damit deterministisch.

use alloc::vec;
use alloc::vec::Vec;

use crate::budget::Budget;
use crate::error::{Result, TensorError};
use crate::layers::{silu, silu_grad};
use crate::loss::cross_entropy as cross_entropy_forward;
use crate::tensor::{byte_len, Tensor};

/// Index eines Knotens im Rechengraphen.
pub type NodeId = usize;

/// Die Operation, die einen Knoten erzeugt hat. Die Eingaben sind Knotenindizes
/// und immer kleiner als der eigene Index; das macht jede Indexreihenfolge zu
/// einer gültigen topologischen Ordnung und den Rückwärtspass zu einer einzigen
/// Schleife ohne Sortierschritt.
#[derive(Debug, Clone)]
enum Op {
    Leaf,
    Add(NodeId, NodeId),
    Mul(NodeId, NodeId),
    Scale(NodeId, f32),
    Matmul(NodeId, NodeId),
    Transpose(NodeId),
    Softmax(NodeId),
    Sum(NodeId),
    RmsNorm { x: NodeId, w: NodeId, eps: f32 },
    SwiGlu(NodeId),
    CrossEntropy { logits: NodeId, targets: Vec<u32> },
}

impl Op {
    fn for_each_input(&self, mut f: impl FnMut(NodeId)) {
        match self {
            Op::Leaf => {}
            Op::Add(a, b) | Op::Mul(a, b) | Op::Matmul(a, b) => {
                f(*a);
                f(*b);
            }
            Op::Scale(a, _) | Op::Transpose(a) | Op::Softmax(a) | Op::Sum(a) | Op::SwiGlu(a) => {
                f(*a)
            }
            Op::RmsNorm { x, w, .. } => {
                f(*x);
                f(*w);
            }
            Op::CrossEntropy { logits, .. } => f(*logits),
        }
    }

    fn is_leaf(&self) -> bool {
        matches!(self, Op::Leaf)
    }
}

struct Node {
    value: Tensor,
    op: Op,
    requires_grad: bool,
}

/// Rechengraph. Besitzt die Vorwärtswerte und die akkumulierten Gradienten
/// seiner Parameter.
pub struct Graph {
    nodes: Vec<Node>,
    /// Je Knoten die Byte, die dieser Graph für den Wert reserviert hat.
    /// Blätter sind 0: ihr Puffer kommt vom Aufrufer und wird hier nicht
    /// verbucht.
    value_bytes: Vec<usize>,
    /// Akkumulierte Gradienten. Nur Parameter (Blätter mit `requires_grad`)
    /// behalten einen Gradienten über den Aufruf hinaus.
    grads: Vec<Option<Tensor>>,
    grad_bytes: Vec<usize>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            value_bytes: Vec::new(),
            grads: Vec::new(),
            grad_bytes: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Blatt ohne Gradientenpflicht, etwa eine Konstante oder ein Zielwert.
    pub fn leaf(&mut self, value: Tensor) -> NodeId {
        self.push(value, Op::Leaf, false, 0)
    }

    /// Blatt mit Gradientenpflicht, also ein trainierbarer Parameter.
    pub fn parameter(&mut self, value: Tensor) -> NodeId {
        self.push(value, Op::Leaf, true, 0)
    }

    fn push(&mut self, value: Tensor, op: Op, requires_grad: bool, bytes: usize) -> NodeId {
        self.nodes.push(Node {
            value,
            op,
            requires_grad,
        });
        self.value_bytes.push(bytes);
        self.grads.push(None);
        self.grad_bytes.push(0);
        self.nodes.len() - 1
    }

    fn node(&self, id: NodeId) -> Result<&Node> {
        self.nodes.get(id).ok_or(TensorError::UnknownNode)
    }

    /// Vorwärtswert eines Knotens.
    pub fn value(&self, id: NodeId) -> Result<&Tensor> {
        Ok(&self.node(id)?.value)
    }

    pub fn requires_grad(&self, id: NodeId) -> Result<bool> {
        Ok(self.node(id)?.requires_grad)
    }

    /// Akkumulierter Gradient eines Parameters. `None`, solange kein
    /// Rückwärtspass ihn erreicht hat.
    pub fn grad(&self, id: NodeId) -> Option<&Tensor> {
        self.grads.get(id).and_then(|g| g.as_ref())
    }

    // ------------------------------------------------------------- Vorwärts

    pub fn add(&mut self, a: NodeId, b: NodeId, budget: &mut Budget) -> Result<NodeId> {
        self.binary_typed(a, b, budget, OpKind::Add)
    }

    pub fn mul(&mut self, a: NodeId, b: NodeId, budget: &mut Budget) -> Result<NodeId> {
        self.binary_typed(a, b, budget, OpKind::Mul)
    }

    pub fn matmul(&mut self, a: NodeId, b: NodeId, budget: &mut Budget) -> Result<NodeId> {
        self.binary_typed(a, b, budget, OpKind::Matmul)
    }

    fn binary_typed(
        &mut self,
        a: NodeId,
        b: NodeId,
        budget: &mut Budget,
        kind: OpKind,
    ) -> Result<NodeId> {
        let ra = self.node(a)?.requires_grad;
        let rb = self.node(b)?.requires_grad;
        let out = {
            let va = &self.nodes[a].value;
            let vb = &self.nodes[b].value;
            match kind {
                OpKind::Add => va.add(vb, budget)?,
                OpKind::Mul => va.mul(vb, budget)?,
                OpKind::Matmul => va.matmul(vb, budget)?,
            }
        };
        let op = match kind {
            OpKind::Add => Op::Add(a, b),
            OpKind::Mul => Op::Mul(a, b),
            OpKind::Matmul => Op::Matmul(a, b),
        };
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, op, ra || rb, bytes))
    }

    pub fn scale(&mut self, a: NodeId, factor: f32, budget: &mut Budget) -> Result<NodeId> {
        let ra = self.node(a)?.requires_grad;
        let out = { self.nodes[a].value.scale(factor, budget)? };
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, Op::Scale(a, factor), ra, bytes))
    }

    pub fn transpose(&mut self, a: NodeId, budget: &mut Budget) -> Result<NodeId> {
        let ra = self.node(a)?.requires_grad;
        let out = { self.nodes[a].value.transpose(budget)? };
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, Op::Transpose(a), ra, bytes))
    }

    pub fn softmax(&mut self, a: NodeId, budget: &mut Budget) -> Result<NodeId> {
        let ra = self.node(a)?.requires_grad;
        let out = { self.nodes[a].value.softmax(budget)? };
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, Op::Softmax(a), ra, bytes))
    }

    /// Summe aller Elemente. Damit lässt sich ein Skalarverlust bilden, ohne
    /// eine zweite Reduktionsoperation einzuführen.
    pub fn sum(&mut self, a: NodeId, budget: &mut Budget) -> Result<NodeId> {
        let ra = self.node(a)?.requires_grad;
        let total = self.nodes[a].value.sum();
        let mut out = Tensor::zeros(Vec::new(), budget)?;
        out.as_mut_slice()[0] = total;
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, Op::Sum(a), ra, bytes))
    }

    pub fn rms_norm(
        &mut self,
        x: NodeId,
        w: NodeId,
        eps: f32,
        budget: &mut Budget,
    ) -> Result<NodeId> {
        let (rx, rw) = (self.node(x)?.requires_grad, self.node(w)?.requires_grad);
        let out = {
            let vx = &self.nodes[x].value;
            let vw = &self.nodes[w].value;
            vx.rms_norm(vw, eps, budget)?
        };
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, Op::RmsNorm { x, w, eps }, rx || rw, bytes))
    }

    pub fn swiglu(&mut self, a: NodeId, budget: &mut Budget) -> Result<NodeId> {
        let ra = self.node(a)?.requires_grad;
        let out = { self.nodes[a].value.swiglu(budget)? };
        let bytes = byte_len(out.shape())?;
        Ok(self.push(out, Op::SwiGlu(a), ra, bytes))
    }

    /// Kreuzentropie als Knoten. Der Verlust ist ein Skalar, die Ableitung
    /// entsteht im Rückwärtspass aus denselben Logits und ist dann exakt
    /// `softmax - target` (B-08).
    pub fn cross_entropy(
        &mut self,
        logits: NodeId,
        targets: &[u32],
        budget: &mut Budget,
    ) -> Result<NodeId> {
        let rl = self.node(logits)?.requires_grad;
        let (loss, _grad) = {
            let vl = &self.nodes[logits].value;
            cross_entropy_forward(vl, targets, budget)?
        };
        // Der Vorwärtsgradient dient nur der Prüfung von B-08 und wird hier
        // nicht gehalten; der Rückwärtspass rechnet ihn neu, damit Graph und
        // Einzelfunktion nicht zwei verschiedene Gradienten führen können.
        let bytes = byte_len(loss.shape())?;
        Ok(self.push(
            loss,
            Op::CrossEntropy {
                logits,
                targets: targets.to_vec(),
            },
            rl,
            bytes,
        ))
    }

    // -------------------------------------------------------------- Rückwärts

    /// Rückwärtspass von `root`. `seed` ist der Gradient am Ausgang; ohne Angabe
    /// ist er 1,0 und der Ausgang muss ein Skalar sein.
    ///
    /// Mehrfacher Aufruf addiert auf die vorhandenen Parametergradienten (B-12).
    pub fn backward(
        &mut self,
        root: NodeId,
        seed: Option<&Tensor>,
        budget: &mut Budget,
    ) -> Result<()> {
        let root_value = self.value(root)?;
        let seed_owned = match seed {
            Some(s) => {
                if !s.same_shape(root_value) {
                    return Err(TensorError::ShapeMismatch);
                }
                s.clone()
            }
            None => {
                if root_value.size() != 1 {
                    return Err(TensorError::ShapeMismatch);
                }
                Tensor::scalar(1.0)
            }
        };

        let (mut local, charges) = self.backward_local(root, &seed_owned, budget)?;

        // Übernahme in den dauerhaften Speicher. Sie allokiert nichts: ein
        // vorhandener Gradient wird in place addiert, ein fehlender übernimmt
        // den Puffer samt Buchung.
        let n = self.nodes.len();
        for id in 0..n {
            let Some(g) = local[id].take() else { continue };
            if self.nodes[id].requires_grad && self.nodes[id].op.is_leaf() {
                match &mut self.grads[id] {
                    Some(dst) => {
                        for (d, s) in dst.as_mut_slice().iter_mut().zip(g.as_slice()) {
                            *d += *s;
                        }
                        budget.release(charges[id]);
                    }
                    None => {
                        self.grads[id] = Some(g);
                        self.grad_bytes[id] += charges[id];
                    }
                }
            } else {
                budget.release(charges[id]);
            }
        }
        Ok(())
    }

    /// Setzt alle Parametergradienten zurück und gibt ihre Bytes frei.
    pub fn zero_grads(&mut self, budget: &mut Budget) {
        for id in 0..self.nodes.len() {
            if self.grads[id].is_some() {
                self.grads[id] = None;
                let b = self.grad_bytes[id];
                budget.release(b);
                self.grad_bytes[id] = 0;
            }
        }
    }

    /// Gibt den gesamten Graphen frei: Werte, Gradienten und Buchungen.
    pub fn clear(&mut self, budget: &mut Budget) {
        for id in 0..self.nodes.len() {
            budget.release(self.value_bytes[id]);
            budget.release(self.grad_bytes[id]);
        }
        self.nodes.clear();
        self.value_bytes.clear();
        self.grads.clear();
        self.grad_bytes.clear();
    }

    /// Summe der Bytes, die dieser Graph aktuell im Budget hält.
    pub fn held_bytes(&self) -> usize {
        self.value_bytes.iter().sum::<usize>() + self.grad_bytes.iter().sum::<usize>()
    }

    /// Lokaler Rückwärtspass. Gibt die Gradienten je Knoten und die je Knoten
    /// reservierte Byte zurück. Bei Fehler ist das Budget wieder auf dem Stand
    /// vor dem Aufruf.
    fn backward_local(
        &self,
        root: NodeId,
        seed: &Tensor,
        budget: &mut Budget,
    ) -> Result<(Vec<Option<Tensor>>, Vec<usize>)> {
        let n = self.nodes.len();
        let mut reachable = vec![false; n];
        let mut stack: Vec<NodeId> = vec![root];
        reachable[root] = true;
        while let Some(id) = stack.pop() {
            self.nodes[id].op.for_each_input(|p| {
                if !reachable[p] {
                    reachable[p] = true;
                    stack.push(p);
                }
            });
        }

        let mut grads: Vec<Option<Tensor>> = (0..n).map(|_| None).collect();
        let mut charges: Vec<usize> = vec![0; n];
        let mut outstanding: usize = 0;

        let res = self.backward_inner(
            root,
            seed,
            &reachable,
            budget,
            &mut grads,
            &mut charges,
            &mut outstanding,
        );
        if let Err(e) = res {
            budget.release(outstanding);
            return Err(e);
        }
        Ok((grads, charges))
    }

    #[allow(clippy::too_many_arguments)]
    fn backward_inner(
        &self,
        root: NodeId,
        seed: &Tensor,
        reachable: &[bool],
        budget: &mut Budget,
        grads: &mut [Option<Tensor>],
        charges: &mut [usize],
        outstanding: &mut usize,
    ) -> Result<()> {
        // Startwert als erster verbuchter Puffer.
        let mut start = new_buf(seed.shape(), budget, outstanding)?;
        start.as_mut_slice().copy_from_slice(seed.as_slice());
        charges[root] = byte_len(seed.shape())?;
        grads[root] = Some(start);

        for id in (0..self.nodes.len()).rev() {
            if !reachable[id] {
                continue;
            }
            let Some(g) = grads[id].take() else { continue };
            let g_bytes = charges[id];
            charges[id] = 0;

            match &self.nodes[id].op {
                Op::Leaf => {
                    // Blätter geben ihren Gradienten weiter; die Übernahme
                    // passiert in `backward`.
                    grads[id] = Some(g);
                    charges[id] = g_bytes;
                    continue;
                }
                Op::Add(a, b) => {
                    let av = &self.nodes[*a].value;
                    let bv = &self.nodes[*b].value;
                    let ga = copy_of(&g, av.shape(), budget, outstanding)?;
                    let gb = copy_of(&g, bv.shape(), budget, outstanding)?;
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                    accumulate(grads, charges, *b, gb, budget, outstanding);
                }
                Op::Mul(a, b) => {
                    let av = &self.nodes[*a].value;
                    let bv = &self.nodes[*b].value;
                    let mut ga = new_buf(av.shape(), budget, outstanding)?;
                    for (o, (gv, x)) in ga
                        .as_mut_slice()
                        .iter_mut()
                        .zip(g.as_slice().iter().zip(bv.as_slice()))
                    {
                        *o = gv * x;
                    }
                    let mut gb = new_buf(bv.shape(), budget, outstanding)?;
                    for (o, (gv, x)) in gb
                        .as_mut_slice()
                        .iter_mut()
                        .zip(g.as_slice().iter().zip(av.as_slice()))
                    {
                        *o = gv * x;
                    }
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                    accumulate(grads, charges, *b, gb, budget, outstanding);
                }
                Op::Scale(a, factor) => {
                    let av = &self.nodes[*a].value;
                    let mut ga = new_buf(av.shape(), budget, outstanding)?;
                    for (o, gv) in ga.as_mut_slice().iter_mut().zip(g.as_slice()) {
                        *o = gv * factor;
                    }
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                }
                Op::Matmul(a, b) => {
                    let av = &self.nodes[*a].value;
                    let bv = &self.nodes[*b].value;
                    let (m, k) = (av.shape()[0], av.shape()[1]);
                    let nn = bv.shape()[1];
                    let ga = matmul_grad_a(&g, bv, m, k, nn, budget, outstanding)?;
                    let gb = matmul_grad_b(&g, av, m, k, nn, budget, outstanding)?;
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                    accumulate(grads, charges, *b, gb, budget, outstanding);
                }
                Op::Transpose(a) => {
                    let av = &self.nodes[*a].value;
                    let (r, c) = (av.shape()[0], av.shape()[1]);
                    let mut ga = new_buf(av.shape(), budget, outstanding)?;
                    {
                        let gs = g.as_slice();
                        let os = ga.as_mut_slice();
                        for i in 0..r {
                            for j in 0..c {
                                os[i * c + j] = gs[j * r + i];
                            }
                        }
                    }
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                }
                Op::Softmax(a) => {
                    // Die Ableitung braucht den Ausgang der Operation, nicht den
                    // Eingang: dv/dz = s * (g - <g, s>). Deshalb der eigene Wert
                    // des Knotens; die Form des Gradienten ist die des Eingangs.
                    let av = &self.nodes[*a].value;
                    let sv = &self.nodes[id].value;
                    let cols = *av.shape().last().unwrap_or(&0);
                    let mut ga = new_buf(av.shape(), budget, outstanding)?;
                    {
                        let y = sv.as_slice();
                        let gs = g.as_slice();
                        let os = ga.as_mut_slice();
                        let rows = av.size().checked_div(cols).unwrap_or(0);
                        for r in 0..rows {
                            let (yr, gr) =
                                (&y[r * cols..(r + 1) * cols], &gs[r * cols..(r + 1) * cols]);
                            let mut dot: f32 = 0.0;
                            for (yi, gi) in yr.iter().zip(gr) {
                                dot += yi * gi;
                            }
                            for c in 0..cols {
                                os[r * cols + c] = yr[c] * (gr[c] - dot);
                            }
                        }
                    }
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                }
                Op::Sum(a) => {
                    let av = &self.nodes[*a].value;
                    let s = g.as_slice()[0];
                    let mut ga = new_buf(av.shape(), budget, outstanding)?;
                    for o in ga.as_mut_slice().iter_mut() {
                        *o = s;
                    }
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                }
                Op::RmsNorm { x, w, eps } => {
                    let xv = &self.nodes[*x].value;
                    let wv = &self.nodes[*w].value;
                    let rank = xv.rank();
                    let d = xv.shape()[rank - 1];
                    let mut gx = new_buf(xv.shape(), budget, outstanding)?;
                    let mut gw = new_buf(wv.shape(), budget, outstanding)?;
                    {
                        let xs = xv.as_slice();
                        let ws = wv.as_slice();
                        let gs = g.as_slice();
                        let gxs = gx.as_mut_slice();
                        let gws = gw.as_mut_slice();
                        for r in 0..xv.size() / d {
                            let row = &xs[r * d..(r + 1) * d];
                            let grow = &gs[r * d..(r + 1) * d];
                            let mut acc: f32 = 0.0;
                            for &v in row {
                                acc += v * v;
                            }
                            let inv = 1.0 / libm::sqrtf(acc / d as f32 + eps);
                            let rms = 1.0 / inv;
                            let rms3 = rms * rms * rms;
                            let mut s: f32 = 0.0;
                            for c in 0..d {
                                s += grow[c] * ws[c] * row[c];
                            }
                            for c in 0..d {
                                gxs[r * d + c] =
                                    grow[c] * ws[c] * inv - (row[c] / (d as f32 * rms3)) * s;
                                gws[c] += grow[c] * row[c] * inv;
                            }
                        }
                    }
                    accumulate(grads, charges, *x, gx, budget, outstanding);
                    accumulate(grads, charges, *w, gw, budget, outstanding);
                }
                Op::SwiGlu(a) => {
                    let av = &self.nodes[*a].value;
                    let (rows, cols) = (av.shape()[0], av.shape()[1]);
                    let h = cols / 2;
                    let mut ga = new_buf(av.shape(), budget, outstanding)?;
                    {
                        let xs = av.as_slice();
                        let gs = g.as_slice();
                        let os = ga.as_mut_slice();
                        for r in 0..rows {
                            for k in 0..h {
                                let a0 = xs[r * cols + k];
                                let b0 = xs[r * cols + h + k];
                                let gv = gs[r * h + k];
                                os[r * cols + k] = gv * b0 * silu_grad(a0);
                                os[r * cols + h + k] = gv * silu(a0);
                            }
                        }
                    }
                    accumulate(grads, charges, *a, ga, budget, outstanding);
                }
                Op::CrossEntropy { logits, targets } => {
                    let lv = &self.nodes[*logits].value;
                    let mut gl = new_buf(lv.shape(), budget, outstanding)?;
                    {
                        let (n, v) = (lv.shape()[0], lv.shape()[1]);
                        let ls = lv.as_slice();
                        let os = gl.as_mut_slice();
                        let scale = g.as_slice()[0];
                        for r in 0..n {
                            let row = &ls[r * v..(r + 1) * v];
                            let mut mx = row[0];
                            for &x in row {
                                if x > mx {
                                    mx = x;
                                }
                            }
                            let mut sum_exp: f32 = 0.0;
                            for &x in row {
                                sum_exp += libm::expf(x - mx);
                            }
                            let t = targets[r] as usize;
                            for c in 0..v {
                                let p = libm::expf(row[c] - mx) / sum_exp;
                                let y = if c == t { 1.0 } else { 0.0 };
                                os[r * v + c] = scale * (p - y);
                            }
                        }
                    }
                    accumulate(grads, charges, *logits, gl, budget, outstanding);
                }
            }

            // Der verbrauchte Gradient dieses Knotens war ein verbuchter
            // Puffer; nach dem Verteilen wird er freigegeben.
            budget.release(g_bytes);
            *outstanding -= g_bytes;
            charges[id] = 0;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum OpKind {
    Add,
    Mul,
    Matmul,
}

/// Neuer, verbuchter Puffer.
fn new_buf(shape: &[usize], budget: &mut Budget, outstanding: &mut usize) -> Result<Tensor> {
    let t = Tensor::zeros(shape.to_vec(), budget)?;
    *outstanding += byte_len(shape)?;
    Ok(t)
}

fn copy_of(
    src: &Tensor,
    shape: &[usize],
    budget: &mut Budget,
    outstanding: &mut usize,
) -> Result<Tensor> {
    let mut t = new_buf(shape, budget, outstanding)?;
    t.as_mut_slice().copy_from_slice(src.as_slice());
    Ok(t)
}

/// `gA = g · Bᵀ`, also `gA[i,k] = Σ_j g[i,j] · B[k,j]`.
fn matmul_grad_a(
    g: &Tensor,
    b: &Tensor,
    m: usize,
    k: usize,
    n: usize,
    budget: &mut Budget,
    outstanding: &mut usize,
) -> Result<Tensor> {
    let mut ga = new_buf(&[m, k], budget, outstanding)?;
    let gs = g.as_slice();
    let bs = b.as_slice();
    let os = ga.as_mut_slice();
    for i in 0..m {
        for x in 0..k {
            let mut acc: f32 = 0.0;
            for j in 0..n {
                acc += gs[i * n + j] * bs[x * n + j];
            }
            os[i * k + x] = acc;
        }
    }
    Ok(ga)
}

/// `gB = Aᵀ · g`, also `gB[k,j] = Σ_i A[i,k] · g[i,j]`.
fn matmul_grad_b(
    g: &Tensor,
    a: &Tensor,
    m: usize,
    k: usize,
    n: usize,
    budget: &mut Budget,
    outstanding: &mut usize,
) -> Result<Tensor> {
    let mut gb = new_buf(&[k, n], budget, outstanding)?;
    let gs = g.as_slice();
    let as_ = a.as_slice();
    let os = gb.as_mut_slice();
    for x in 0..k {
        for j in 0..n {
            let mut acc: f32 = 0.0;
            for i in 0..m {
                acc += as_[i * k + x] * gs[i * n + j];
            }
            os[x * n + j] = acc;
        }
    }
    Ok(gb)
}

/// Addiert einen Beitrag auf den vorhandenen Gradienten eines Knotens.
///
/// Wird der Beitrag in einen bestehenden Puffer addiert, ist sein eigener
/// Puffer danach frei; seine Buchung muss deshalb zurückgegeben werden. Wird er
/// übernommen, bleibt die Buchung bestehen und der Puffer gehört dem Knoten.
fn accumulate(
    grads: &mut [Option<Tensor>],
    charges: &mut [usize],
    id: NodeId,
    contrib: Tensor,
    budget: &mut Budget,
    outstanding: &mut usize,
) {
    let bytes = byte_len(contrib.shape()).unwrap_or(0);
    match &mut grads[id] {
        Some(dst) => {
            for (d, s) in dst.as_mut_slice().iter_mut().zip(contrib.as_slice()) {
                *d += *s;
            }
            budget.release(bytes);
            *outstanding -= bytes;
        }
        None => {
            grads[id] = Some(contrib);
            // Buchung nicht vergessen: sonst gibt `clear` die Byte dieses
            // Beitrags nie zurueck und das Budget laeuft voll.
            charges[id] = bytes;
        }
    }
}

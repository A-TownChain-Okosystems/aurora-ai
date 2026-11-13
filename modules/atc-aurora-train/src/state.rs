// atc-aurora-train — Pruefpunktzustand (M-06)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Format `AURST001`. Es traegt die Information, die ein Artefakt (ATCM) nicht
// traegt: Schritt, gesehene Token, Hyperparameter und die Momente des
// Optimierers. Ohne diese Momente ist ein fortgesetzter Lauf nicht derselbe
// Lauf, sondern ein neuer mit denselben Gewichten.
//
// Kodierung: Little-Endian, feste Feldreihenfolge, Laengen vor Inhalt. Die
// Pruefsumme steht am Ende und deckt alles davor ab (Digest-Autoritaet
// `atc-aurora-digest`, AD-FT1). Ein dekodierter Zustand ohne passende
// Pruefsumme wird abgelehnt, nicht repariert.

use atc_aurora_digest::{digest, DIGEST_LEN};

use crate::error::TrainError;
use crate::optim::HyperParams;

/// Kennung des Zustandsformats.
pub const STATE_MAGIC: [u8; 8] = *b"AURST001";
/// Fassung des Zustandsformats.
pub const STATE_VERSION: u32 = 1;

/// Alles, was ein fortgesetzter Lauf braucht.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainerState {
    /// Abgeschlossene Optimierungsschritte.
    pub step: u64,
    /// Bisher gesehene Zielpositionen (Token).
    pub tokens_seen: u64,
    /// Hyperparameter des Laufs.
    pub hp: HyperParams,
    /// Schrittzaehler des Optimierers (Bias-Korrektur).
    pub adam_t: u64,
    /// Erstes Moment je Parameter.
    pub m: Vec<Vec<f32>>,
    /// Zweites Moment je Parameter.
    pub v: Vec<Vec<f32>>,
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Kodiert den Zustand samt Pruefsumme.
pub fn encode(state: &TrainerState) -> Vec<u8> {
    let mut out = Vec::with_capacity(128 + 8 * state.m.iter().map(|b| b.len()).sum::<usize>());
    out.extend_from_slice(&STATE_MAGIC);
    put_u32(&mut out, STATE_VERSION);
    put_u64(&mut out, state.step);
    put_u64(&mut out, state.tokens_seen);
    put_f32(&mut out, state.hp.lr);
    put_f32(&mut out, state.hp.beta1);
    put_f32(&mut out, state.hp.beta2);
    put_f32(&mut out, state.hp.eps);
    put_f32(&mut out, state.hp.weight_decay);
    put_f32(&mut out, state.hp.momentum);
    match state.hp.clip_norm {
        Some(c) => {
            out.push(1);
            put_f32(&mut out, c);
        }
        None => out.push(0),
    }
    put_u32(&mut out, state.hp.budget_bytes as u32);
    put_u64(&mut out, state.adam_t);
    put_u32(&mut out, state.m.len() as u32);
    for (i, mb) in state.m.iter().enumerate() {
        let vb = state.v.get(i).map(|b| b.as_slice()).unwrap_or(&[]);
        put_u32(&mut out, mb.len() as u32);
        for f in mb {
            put_f32(&mut out, *f);
        }
        put_u32(&mut out, vb.len() as u32);
        for f in vb {
            put_f32(&mut out, *f);
        }
    }
    let d = digest(&out);
    out.extend_from_slice(&d);
    out
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn need(&self, len: usize) -> Result<(), TrainError> {
        if self.at + len > self.bytes.len() {
            return Err(TrainError::CorruptState { what: "truncated" });
        }
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, TrainError> {
        self.need(1)?;
        let v = self.bytes[self.at];
        self.at += 1;
        Ok(v)
    }

    fn u32(&mut self) -> Result<u32, TrainError> {
        self.need(4)?;
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.bytes[self.at..self.at + 4]);
        self.at += 4;
        Ok(u32::from_le_bytes(b))
    }

    fn u64(&mut self) -> Result<u64, TrainError> {
        self.need(8)?;
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.bytes[self.at..self.at + 8]);
        self.at += 8;
        Ok(u64::from_le_bytes(b))
    }

    fn f32(&mut self) -> Result<f32, TrainError> {
        self.need(4)?;
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.bytes[self.at..self.at + 4]);
        self.at += 4;
        Ok(f32::from_le_bytes(b))
    }
}

/// Dekodiert einen Zustand und prueft die Pruefsumme.
///
/// `param_count` ist die erwartete Anzahl Parameter; der Zustand muss genau so
/// viele Momentpaare tragen, sonst gehoert er zu anderen Gewichten.
pub fn decode(bytes: &[u8], param_count: usize) -> Result<TrainerState, TrainError> {
    if bytes.len() < STATE_MAGIC.len() + DIGEST_LEN {
        return Err(TrainError::CorruptState { what: "truncated" });
    }
    let split = bytes.len() - DIGEST_LEN;
    let (body, tail) = bytes.split_at(split);
    let mut want = [0u8; DIGEST_LEN];
    want.copy_from_slice(tail);
    if digest(body) != want {
        return Err(TrainError::CorruptState { what: "digest" });
    }

    let mut c = Cursor { bytes: body, at: 0 };
    c.need(STATE_MAGIC.len())?;
    if body[..STATE_MAGIC.len()] != STATE_MAGIC {
        return Err(TrainError::CorruptState { what: "magic" });
    }
    c.at = STATE_MAGIC.len();
    if c.u32()? != STATE_VERSION {
        return Err(TrainError::CorruptState { what: "version" });
    }
    let step = c.u64()?;
    let tokens_seen = c.u64()?;
    let lr = c.f32()?;
    let beta1 = c.f32()?;
    let beta2 = c.f32()?;
    let eps = c.f32()?;
    let weight_decay = c.f32()?;
    let momentum = c.f32()?;
    let clip_norm = match c.u8()? {
        0 => None,
        1 => Some(c.f32()?),
        _ => return Err(TrainError::CorruptState { what: "clip_norm" }),
    };
    let budget_bytes = c.u32()? as usize;
    let adam_t = c.u64()?;
    let count = c.u32()? as usize;
    if count != param_count {
        return Err(TrainError::CorruptState {
            what: "param_count",
        });
    }
    let mut m = Vec::with_capacity(count);
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        let ml = c.u32()? as usize;
        let mut mb = Vec::with_capacity(ml);
        for _ in 0..ml {
            mb.push(c.f32()?);
        }
        let vl = c.u32()? as usize;
        let mut vb = Vec::with_capacity(vl);
        for _ in 0..vl {
            vb.push(c.f32()?);
        }
        m.push(mb);
        v.push(vb);
    }
    if c.at != body.len() {
        return Err(TrainError::CorruptState { what: "trailing" });
    }

    let state = TrainerState {
        step,
        tokens_seen,
        hp: HyperParams {
            lr,
            beta1,
            beta2,
            eps,
            weight_decay,
            momentum,
            clip_norm,
            budget_bytes,
        },
        adam_t,
        m,
        v,
    };
    state.hp.validate()?;
    Ok(state)
}

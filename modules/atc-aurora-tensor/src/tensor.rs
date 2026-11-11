// atc-aurora-tensor — Datentyp, Form und Speicher (M-01)
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Der Puffer ist f32. Der `Dtype` beschreibt diesen Puffer, er ist keine
// Dekoration: es gibt genau eine Variante, und sie entspricht dem, was
// tatsächlich im Speicher liegt. Weitere Typen kommen erst, wenn ein Contract
// sie verlangt, und dann als echter zweiter Puffer, nicht als Etikett.

use alloc::vec;
use alloc::vec::Vec;

use crate::budget::Budget;
use crate::error::{Result, TensorError};

/// Byte je Element im Puffer.
pub const BYTES_PER_ELEMENT: usize = core::mem::size_of::<f32>();

/// Der Datentyp des Puffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dtype {
    F32,
}

impl Dtype {
    pub const fn byte_width(self) -> usize {
        match self {
            Dtype::F32 => BYTES_PER_ELEMENT,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Dtype::F32 => "f32",
        }
    }
}

/// Verbindlicher Datentyp der Referenzimplementierung.
pub const DTYPE: Dtype = Dtype::F32;

/// Ein dicht gepackter Tensor, zeilenweise, ohne Strides.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    shape: Vec<usize>,
    data: Vec<f32>,
}

/// Anzahl Elemente einer Form. Geprüft, damit eine große Form nicht still
/// überläuft.
pub fn numel(shape: &[usize]) -> Result<usize> {
    let mut n: usize = 1;
    for &d in shape {
        n = n.checked_mul(d).ok_or(TensorError::SizeOverflow)?;
    }
    Ok(n)
}

/// Bytegröße einer Form. Geprüft wie `numel`, zusätzlich die Multiplikation mit
/// der Elementbreite.
pub fn byte_len(shape: &[usize]) -> Result<usize> {
    numel(shape)?
        .checked_mul(BYTES_PER_ELEMENT)
        .ok_or(TensorError::SizeOverflow)
}

/// Prüft das Budget und teilt danach erst den Puffer zu.
///
/// Reihenfolge ist Teil des Vertrags (B-03): erst Form und Bytegröße prüfen,
/// dann `reserve`, dann `Vec` anlegen. Schlägt `reserve` fehl, wurde nichts
/// zerteilt und nichts verbucht.
pub(crate) fn alloc_zeroed(n: usize, budget: &mut Budget) -> Result<Vec<f32>> {
    let bytes = n
        .checked_mul(BYTES_PER_ELEMENT)
        .ok_or(TensorError::SizeOverflow)?;
    budget.reserve(bytes)?;
    let mut data: Vec<f32> = Vec::new();
    data.reserve_exact(n);
    data.resize(n, 0.0);
    Ok(data)
}

impl Tensor {
    /// Tensor aus vorhandenen Daten. Die Form muss zur Datenlänge passen.
    pub fn new(shape: Vec<usize>, data: Vec<f32>) -> Result<Self> {
        let n = numel(&shape)?;
        if n != data.len() {
            return Err(TensorError::ShapeMismatch);
        }
        Ok(Tensor { shape, data })
    }

    /// Nullinitialisierter Tensor, Budget wird vor der Zuteilung geprüft.
    pub fn zeros(shape: Vec<usize>, budget: &mut Budget) -> Result<Self> {
        let n = numel(&shape)?;
        let data = alloc_zeroed(n, budget)?;
        Ok(Tensor { shape, data })
    }

    /// Rang-1-Tensor aus Vektor.
    pub fn from_vec(data: Vec<f32>) -> Self {
        let shape = vec![data.len()];
        Tensor { shape, data }
    }

    /// Skalar, also Rang 0 mit einem Element.
    pub fn scalar(v: f32) -> Self {
        Tensor {
            shape: Vec::new(),
            data: vec![v],
        }
    }

    /// Konstruktion aus Little-Endian-Bytes eines f32-Puffers (B-04).
    ///
    /// Die Länge muss exakt `4 * numel` sein. Länge 0 ist zulässig und ergibt
    /// einen leeren Tensor. Jede andere Länge ist ein `ShapeMismatch`, wie der
    /// Testfall es verlangt.
    pub fn from_le_bytes(shape: Vec<usize>, bytes: &[u8], budget: &mut Budget) -> Result<Self> {
        let n = numel(&shape)?;
        let expected = n
            .checked_mul(BYTES_PER_ELEMENT)
            .ok_or(TensorError::SizeOverflow)?;
        if bytes.len() != expected {
            return Err(TensorError::ShapeMismatch);
        }
        let mut data = alloc_zeroed(n, budget)?;
        for (i, slot) in data.iter_mut().enumerate() {
            let o = i * BYTES_PER_ELEMENT;
            *slot = f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
        }
        Ok(Tensor { shape, data })
    }

    /// Form des Tensors.
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// Anzahl Elemente.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn dtype(&self) -> Dtype {
        DTYPE
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Schreibzugriff auf den Puffer. Nur für Aufrufer gedacht, die die Form
    /// unverändert lassen; die Länge ist durch die Form festgelegt.
    pub(crate) fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn to_vec(&self) -> Vec<f32> {
        self.data.clone()
    }

    pub fn into_vec(self) -> Vec<f32> {
        self.data
    }

    /// Zwei Tensoren gelten als formgleich, wenn Rang und alle Dimensionen
    /// übereinstimmen. Gleiche Elementzahl bei anderer Form ist keine
    /// Übereinstimmung, es gibt kein implizites Umformen.
    pub fn same_shape(&self, other: &Self) -> bool {
        self.shape == other.shape
    }
}

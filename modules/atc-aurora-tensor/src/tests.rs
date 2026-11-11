// atc-aurora-tensor — Tests
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Zwei Gruppen:
//   1. Migrationsbelege `mig_*`: die Tensor- und Operationstests aus
//      globus-os/modules/atc-shivacore/service_space/src/ai.rs, auf f32
//      übertragen. Sie sichern, dass die Semantik beim Umzug erhalten bleibt.
//   2. Vertragstests `b_*`: die P2-Testfälle B-01 bis B-04 aus
//      ATC-AI-LLM-001-B. Sie sind der eigentliche Nachweis für M-01.
//
// f32 ist der verbindliche Typ, deshalb sind die Toleranzen der Migrationsbelege
// weiter als die alten f64-Werte. Wo das der Fall ist, steht es am Test.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::{Budget, Dtype, Tensor, TensorError, BYTES_PER_ELEMENT, DTYPE};

fn budget() -> Budget {
    Budget::new(1 << 20)
}

// ---------------------------------------------------------------- Migration

#[test]
fn mig_tensor_create() {
    let t = Tensor::new(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    assert_eq!(t.shape(), &[2, 3]);
    assert_eq!(t.size(), 6);
    assert_eq!(t.rank(), 2);
}

#[test]
fn mig_tensor_zeros() {
    let z = Tensor::zeros(vec![3], &mut budget()).unwrap();
    assert_eq!(z.as_slice(), &[0.0, 0.0, 0.0]);
}

#[test]
fn mig_tensor_add() {
    let mut b = budget();
    let a = Tensor::from_vec(vec![1.0, 2.0, 3.0]);
    let o = Tensor::from_vec(vec![4.0, 5.0, 6.0]);
    assert_eq!(a.add(&o, &mut b).unwrap().as_slice(), &[5.0, 7.0, 9.0]);
}

#[test]
fn mig_tensor_mul() {
    let mut b = budget();
    let a = Tensor::from_vec(vec![2.0, 3.0]);
    let o = Tensor::from_vec(vec![4.0, 5.0]);
    assert_eq!(a.mul(&o, &mut b).unwrap().as_slice(), &[8.0, 15.0]);
}

#[test]
fn mig_tensor_scale() {
    let mut b = budget();
    let a = Tensor::from_vec(vec![1.0, 2.0, 3.0]);
    assert_eq!(a.scale(2.0, &mut b).unwrap().as_slice(), &[2.0, 4.0, 6.0]);
}

#[test]
fn mig_tensor_matmul() {
    let mut b = budget();
    let a = Tensor::new(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let o = Tensor::new(vec![3, 2], vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
    let c = a.matmul(&o, &mut b).unwrap();
    assert_eq!(c.shape(), &[2, 2]);
    assert_eq!(c.as_slice(), &[58.0, 64.0, 139.0, 154.0]);
}

#[test]
fn mig_tensor_relu() {
    let mut b = budget();
    let a = Tensor::from_vec(vec![-1.0, 0.0, 2.0, -3.0, 5.0]);
    assert_eq!(
        a.relu(&mut b).unwrap().as_slice(),
        &[0.0, 0.0, 2.0, 0.0, 5.0]
    );
}

#[test]
fn mig_tensor_sigmoid() {
    let mut b = budget();
    let a = Tensor::scalar(0.0);
    // f32 statt f64: Toleranz angepasst, Erwartung unverändert.
    assert!((a.sigmoid(&mut b).unwrap().as_slice()[0] - 0.5).abs() < 1e-6);
}

#[test]
fn mig_tensor_softmax() {
    let mut b = budget();
    let a = Tensor::from_vec(vec![1.0, 2.0, 3.0]);
    let s = a.softmax(&mut b).unwrap();
    assert!((s.sum() - 1.0).abs() < 1e-6);
}

#[test]
fn mig_tensor_transpose() {
    let mut b = budget();
    let a = Tensor::new(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
    let t = a.transpose(&mut b).unwrap();
    assert_eq!(t.shape(), &[3, 2]);
    assert_eq!(t.as_slice(), &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
}

#[test]
fn mig_tensor_dot() {
    let a = Tensor::from_vec(vec![1.0, 2.0, 3.0]);
    let o = Tensor::from_vec(vec![4.0, 5.0, 6.0]);
    assert_eq!(a.dot(&o).unwrap(), 32.0);
}

#[test]
fn mig_tensor_mean() {
    let a = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(a.mean(), 2.5);
}

// ------------------------------------------------------------ Vertrag B-01

#[test]
fn b_01_matmul_unpassende_formen() {
    let mut b = budget();
    let a = Tensor::new(vec![2, 3], vec![0.0; 6]).unwrap();
    let o = Tensor::new(vec![2, 3], vec![0.0; 6]).unwrap();
    assert_eq!(a.matmul(&o, &mut b), Err(TensorError::ShapeMismatch));
    // Nichts zerteilt, nichts verbucht.
    assert_eq!(b.used(), 0);
}

#[test]
fn b_01_matmul_falscher_rang() {
    let mut b = budget();
    let a = Tensor::new(vec![2, 2, 2], vec![0.0; 8]).unwrap();
    let o = Tensor::new(vec![2, 2], vec![0.0; 4]).unwrap();
    assert_eq!(
        a.matmul(&o, &mut b),
        Err(TensorError::UnsupportedRank {
            got: 3,
            expected: "rank 2"
        })
    );
}

// ------------------------------------------------------------ Vertrag B-02

#[test]
fn b_02_addition_ohne_broadcasting() {
    let mut b = budget();
    let a = Tensor::new(vec![3, 1], vec![1.0, 2.0, 3.0]).unwrap();
    let o = Tensor::new(vec![1, 4], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    assert_eq!(a.add(&o, &mut b), Err(TensorError::ShapeMismatch));
}

#[test]
fn b_02_gleiche_elementzahl_ist_keine_gleiche_form() {
    let mut b = budget();
    let a = Tensor::new(vec![2, 3], vec![1.0; 6]).unwrap();
    let o = Tensor::new(vec![3, 2], vec![1.0; 6]).unwrap();
    assert_eq!(a.add(&o, &mut b), Err(TensorError::ShapeMismatch));
}

// ------------------------------------------------------------ Vertrag B-03

#[test]
fn b_03_budget_vor_der_zuteilung() {
    let mut b = Budget::new(0);
    let err = Tensor::zeros(vec![2, 2], &mut b);
    assert_eq!(
        err,
        Err(TensorError::BudgetExceeded {
            requested: 16,
            limit: 0
        })
    );
    // Der Beweis, dass vor der Zuteilung geprüft wurde: nichts ist verbucht.
    assert_eq!(b.used(), 0);
}

#[test]
fn b_03_budget_grenze_exakt() {
    let mut b = Budget::new(16);
    assert!(Tensor::zeros(vec![2, 2], &mut b).is_ok());
    assert_eq!(b.used(), 16);
    // Zweiter Puffer derselben Größe passt nicht mehr.
    assert_eq!(
        Tensor::zeros(vec![2, 2], &mut b),
        Err(TensorError::BudgetExceeded {
            requested: 16,
            limit: 16
        })
    );
    assert_eq!(b.used(), 16);
}

#[test]
fn b_03_operation_respektiert_budget() {
    let mut b = Budget::new(12);
    let a = Tensor::from_vec(vec![1.0, 2.0, 3.0]);
    let o = Tensor::from_vec(vec![1.0, 2.0, 3.0]);
    // 12 Bytes reichen für genau eine Ausgabe von drei f32.
    assert!(a.add(&o, &mut b).is_ok());
    assert_eq!(
        a.add(&o, &mut b),
        Err(TensorError::BudgetExceeded {
            requested: 12,
            limit: 12
        })
    );
}

// ------------------------------------------------------------ Vertrag B-04

#[test]
fn b_04_bytes_little_endian() {
    let mut b = budget();
    let mut bytes = Vec::new();
    for v in [1.0f32, -2.5, 0.25] {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    let t = Tensor::from_le_bytes(vec![3], &bytes, &mut b).unwrap();
    assert_eq!(t.as_slice(), &[1.0, -2.5, 0.25]);
}

#[test]
fn b_04_leere_laenge_zulaessig() {
    let mut b = budget();
    let t = Tensor::from_le_bytes(vec![0], &[], &mut b).unwrap();
    assert_eq!(t.size(), 0);
    assert_eq!(t.rank(), 1);
}

#[test]
fn b_04_falsche_laenge() {
    let mut b = budget();
    // 3 Bytes sind für ein f32 zu wenig, 5 zu viel.
    assert_eq!(
        Tensor::from_le_bytes(vec![1], &[1, 2, 3], &mut b),
        Err(TensorError::ShapeMismatch)
    );
    assert_eq!(
        Tensor::from_le_bytes(vec![1], &[1, 2, 3, 4, 5], &mut b),
        Err(TensorError::ShapeMismatch)
    );
}

// ------------------------------------------------------- Grenzwerte, Formen

#[test]
fn form_ueberlauf_wird_erkannt() {
    let err = crate::tensor::numel(&[usize::MAX, 2]);
    assert_eq!(err, Err(TensorError::SizeOverflow));
    let mut b = budget();
    assert_eq!(
        Tensor::zeros(vec![usize::MAX, 2], &mut b),
        Err(TensorError::SizeOverflow)
    );
    assert_eq!(b.used(), 0);
}

#[test]
fn bytegroesse_ueberlauf_wird_erkannt() {
    // numel passt noch in usize, die Bytegröße nicht mehr.
    let n = usize::MAX / 2;
    let mut b = Budget::unlimited();
    let r = Tensor::zeros(vec![n], &mut b);
    assert!(matches!(r, Err(TensorError::SizeOverflow)));
    assert_eq!(b.used(), 0);
}

#[test]
fn datentyp_ist_echt() {
    let t = Tensor::from_vec(vec![1.0, 2.0]);
    assert_eq!(t.dtype(), Dtype::F32);
    assert_eq!(t.dtype(), DTYPE);
    assert_eq!(DTYPE.byte_width(), BYTES_PER_ELEMENT);
    assert_eq!(BYTES_PER_ELEMENT, 4);
    assert_eq!(crate::tensor::byte_len(&[2, 3]).unwrap(), 24);
}

#[test]
fn datenlaenge_muss_zur_form_passen() {
    assert_eq!(
        Tensor::new(vec![2, 3], vec![0.0; 5]),
        Err(TensorError::ShapeMismatch)
    );
}

#[test]
fn softmax_stabil_bei_grossen_werten() {
    let mut b = budget();
    let a = Tensor::from_vec(vec![1000.0, 1001.0, 1002.0]);
    let s = a.softmax(&mut b).unwrap();
    for v in s.as_slice() {
        assert!(v.is_finite(), "softmax darf nicht überlaufen: {v}");
    }
    assert!((s.sum() - 1.0).abs() < 1e-6);
}

#[test]
fn softmax_zeilenweise_zweidimensional() {
    let mut b = budget();
    let a = Tensor::new(vec![2, 2], vec![0.0, 0.0, 1.0, 1.0]).unwrap();
    let s = a.softmax(&mut b).unwrap();
    let d = s.as_slice();
    assert!((d[0] - 0.5).abs() < 1e-6);
    assert!((d[1] - 0.5).abs() < 1e-6);
    assert!((d[2] + d[3] - 1.0).abs() < 1e-6);
}

#[test]
fn budget_freigabe_ermoeglicht_neue_zuteilung() {
    let mut b = Budget::new(8);
    let t = Tensor::zeros(vec![2], &mut b).unwrap();
    assert_eq!(b.used(), 8);
    b.release(8);
    assert_eq!(b.used(), 0);
    assert!(Tensor::zeros(vec![2], &mut b).is_ok());
    assert_eq!(t.size(), 2);
}

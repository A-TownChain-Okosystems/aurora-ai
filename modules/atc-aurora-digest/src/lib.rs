// atc-aurora-digest — SHA-256 als einzige Digest-Autoritaet, AD-FT1
// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
//
// Eigentuemer der Pruefsumme fuer alle Aurora-Artefakte. Vor AD-FT1 gab es zwei
// Implementierungen (`atc-aurora-model/src/artifact/sha256.rs` und im Tokenizer),
// bytegenau gleich, aber doppelt gepflegt. Seit AD-FT1 gibt es genau eine.
//
// Bindend:
//   1. Keine Abhaengigkeit und kein `unsafe`. Die Pruefsumme eines Artefakts
//      haengt an einem Algorithmus, den niemand von aussen austauschen darf.
//   2. Streaming ist die normative Form: konstanter Zustand, kein Puffer fuer die
//      gesamte Eingabe. Die Einmal-APIs sind Bequemlichkeit ueber derselben
//      Zustandsmaschine, nicht ein zweiter Rechenweg.
//   3. Bytegenaue Gleichheit gegen einen unabhaengigen Orakel-Vektor
//      (`tests/vectors.txt`, erzeugt mit python hashlib) und gegen die
//      FIPS-180-4-Beispiele. Zusaetzlich pruefen Model und Tokenizer denselben
//      Vektor gegen ihre eigenen Fassaden (Cross-Crate-Test).
//   4. Das Crate baut ohne std. Hex-Ausgabe braucht `alloc`, sonst nichts.

#![forbid(unsafe_code)]
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

/// Kennung und Fassung des API-Vertrags (`ATC-AI-DIGEST-001`).
pub const API_VERSION: &str = "ATC-AI-DIGEST-001 v1.0.0";

/// Laenge eines Digests in Bytes.
pub const DIGEST_LEN: usize = 32;

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Laufender SHA-256-Zustand. Ein Durchlauf, keine Zwischenpufferung des
/// gesamten Eingabestroms: der Aufrufer schiebt Stuecke hinein.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    block_len: usize,
    total_len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    /// Frischer Zustand nach FIPS 180-4.
    pub const fn new() -> Self {
        Sha256 {
            state: H0,
            block: [0u8; 64],
            block_len: 0,
            total_len: 0,
        }
    }

    /// Schiebt Daten hinein. Der Aufwand ist linear, der Zustand konstant.
    pub fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);
        while !data.is_empty() {
            let take = core::cmp::min(64 - self.block_len, data.len());
            self.block[self.block_len..self.block_len + take].copy_from_slice(&data[..take]);
            self.block_len += take;
            data = &data[take..];
            if self.block_len == 64 {
                let block = self.block;
                self.compress(&block);
                self.block_len = 0;
            }
        }
    }

    /// Schliesst ab und liefert den Digest. Ein zweiter Aufruf ohne `update`
    /// ergaebe einen anderen Wert; der Aufrufer verwendet die Instanz danach
    /// nicht weiter.
    pub fn finish(mut self) -> [u8; DIGEST_LEN] {
        let bit_len = self.total_len.wrapping_mul(8);
        self.update_no_len(&[0x80]);
        while self.block_len != 56 {
            self.update_no_len(&[0x00]);
        }
        self.update_no_len(&bit_len.to_be_bytes());
        debug_assert_eq!(self.block_len, 0);
        let mut out = [0u8; DIGEST_LEN];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn update_no_len(&mut self, data: &[u8]) {
        let saved = self.total_len;
        self.update(data);
        self.total_len = saved;
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }
}

/// SHA-256 ueber einen vollstaendigen Puffer. Baut **keine** Kopie der Eingabe;
/// der Puffer ist bereits der Strom, der durch die Zustandsmaschine laeuft.
pub fn digest(data: &[u8]) -> [u8; DIGEST_LEN] {
    let mut h = Sha256::new();
    h.update(data);
    h.finish()
}

/// Wie [`digest`], unter dem Namen der frueheren Tokenizer-Fassade.
pub fn sha256(data: &[u8]) -> [u8; DIGEST_LEN] {
    digest(data)
}

/// Digest als 64 Zeichen kleingeschriebenes Hex (Format nach `ATC-AI-LLM-004` C-22).
pub fn hex(digest: &[u8; DIGEST_LEN]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// SHA-256 ueber `data` als Hex-Digest.
pub fn sha256_hex(data: &[u8]) -> String {
    hex(&digest(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_version_ist_gesetzt() {
        assert_eq!(API_VERSION, "ATC-AI-DIGEST-001 v1.0.0");
        assert_eq!(DIGEST_LEN, 32);
    }
}

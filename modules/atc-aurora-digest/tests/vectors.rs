//! Bytegenaue Pruefung gegen einen unabhaengigen Orakel-Vektor.
//!
//! `vectors.txt` wurde nicht aus diesem Code erzeugt, sondern mit python hashlib
//! ueber ein festes Byte-Muster in 301 Laengen. Damit ist die Pruefung nicht
//! zirkulaer: sie vergleicht die Umsetzung mit einer fremden.

use atc_aurora_digest::{digest, hex, sha256, sha256_hex, Sha256};

const VEKTOREN: &str = include_str!("vectors.txt");

/// Muster der Vektoren: Byte i = (i*37+11) mod 256.
fn muster(laenge: usize) -> Vec<u8> {
    (0..laenge).map(|i| ((i * 37 + 11) % 256) as u8).collect()
}

fn erwartungen() -> Vec<(usize, String)> {
    VEKTOREN
        .lines()
        .filter(|z| !z.starts_with('#') && !z.trim().is_empty())
        .map(|z| {
            let mut teile = z.split_whitespace();
            let laenge: usize = teile.next().expect("Laenge").parse().expect("Zahl");
            let hexwert = teile.next().expect("Digest").to_string();
            (laenge, hexwert)
        })
        .collect()
}

#[test]
fn alle_vektoren_laengen_null_bis_dreihundert() {
    let vektoren = erwartungen();
    assert_eq!(vektoren.len(), 301, "Vektorliste unvollstaendig");
    for (laenge, erwartet) in vektoren {
        let daten = muster(laenge);
        assert_eq!(&sha256_hex(&daten), &erwartet, "Laenge {laenge}");
        assert_eq!(&hex(&digest(&daten)), &erwartet, "Laenge {laenge} (digest)");
        assert_eq!(&hex(&sha256(&daten)), &erwartet, "Laenge {laenge} (sha256)");
    }
}

#[test]
fn streaming_ergibt_denselben_digest_wie_ein_stueck() {
    // Unregelmaessige Portionen, inklusive Portionen groesser als ein Block und
    // einer leeren Portion: der Zustand darf daran nichts aendern.
    let schrittweiten = [1usize, 3, 7, 32, 63, 64, 65, 100, 127, 128, 255];
    for laenge in [0usize, 1, 55, 56, 57, 63, 64, 65, 119, 120, 127, 128, 129, 300] {
        let daten = muster(laenge);
        let erwartet = hex(&digest(&daten));
        for schritt in schrittweiten {
            let mut h = Sha256::new();
            let mut rest: &[u8] = &daten;
            h.update(&[]);
            while !rest.is_empty() {
                let take = core::cmp::min(schritt, rest.len());
                h.update(&rest[..take]);
                rest = &rest[take..];
            }
            assert_eq!(hex(&h.finish()), erwartet, "Laenge {laenge}, Schritt {schritt}");
        }
    }
}

#[test]
fn fips_180_4_beispiele() {
    // Offizielle Beispiele aus FIPS 180-4 / den NIST-Testvektoren.
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn hex_ist_kleingeschrieben_und_64_stellen() {
    let wert = sha256_hex(b"aurora");
    assert_eq!(wert.len(), 64);
    assert!(wert.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
}

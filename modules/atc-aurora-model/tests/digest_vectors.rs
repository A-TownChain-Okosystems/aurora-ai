//! Cross-Crate-Nachweis (AD-FT1): diese Fassade liefert bytegenau dieselben
//! Digests wie der unabhaengige Orakel-Vektor der Digest-Autoritaet.
//!
//! Der Vektor wurde mit python hashlib erzeugt, nicht aus dem Rust-Code. Damit
//! belegt der Test nicht nur, dass beide Crates denselben Code benutzen, sondern
//! dass dieser Code richtig rechnet.

const VEKTOREN: &str = include_str!("../../atc-aurora-digest/tests/vectors.txt");

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
            (laenge, teile.next().expect("Digest").to_string())
        })
        .collect()
}

#[test]
fn fassade_trifft_den_orakel_vektor() {
    let vektoren = erwartungen();
    assert_eq!(vektoren.len(), 301);
    for (laenge, erwartet) in vektoren {
        let daten = muster(laenge);
        let hexwert = atc_aurora_model::artifact::sha256::digest(&daten)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(hexwert, erwartet, "Laenge {laenge}");
    }
}

# Status atc-aurora-tokenizer (M-04)

| Bereich | Status | Beleg |
|---|---|---|
| Training (Byte-Level-BPE) | `IMPLEMENTED` | `Bpe::train`, Häufigkeit absteigend, Tie-Break lexikografisch kleinste Paarung, Abbruch bei Einmal-Paarungen |
| Kodieren / Dekodieren | `IMPLEMENTED` | `encode` (Rang-Minimum zuerst, überlappungsfrei), `decode` (typisierter Fehler bei unbekannter ID) |
| Artefakt / Digest | `IMPLEMENTED` | `canonical`, `digest_hex`, `from_canonical` mit Prüfung **vor** dem Parsen |
| Lokale Prüfung | `TESTED` | 15 Tests grün (stable und nightly), clippy `-D warnings` sauber, fmt sauber |
| `CI_VERIFIED` | **offen** | Actions endet mit `startup_failure` ohne Job, Step und Log |
| `E2E_VERIFIED` | **offen** | kein Lauf im Gesamtsystem, kein trainierter Korpus |

## Blueprint-Testfälle

| ID | Prüfung | Stand |
|---|---|---|
| B-14 | Rundlauf über 10 000 Zufallsfolgen inkl. ungültigem UTF-8 | `tests::b_14_rundlauf_ueber_zehntausend_zufallsfolgen`, `tests::b_14_rundlauf_ueber_text_und_zufall_gemischt` |
| B-15 | zwei Trainingsläufe, identisches Vokabular und identischer Digest | `tests::b_15_zwei_laeufe_ergeben_identisches_vokabular_und_digest`, `tests::b_15_merkmal_stabil_ueber_wiederholte_laeufe` |
| B-16 | Grenzfälle: leer, ein Byte, Emoji, 4-Byte-Sequenz, 1 MB Zufall | `tests::b_16_grenzfaelle_ohne_panik_und_ohne_ersatzzeichen`, `tests::zwanzig_testvektoren_rundlauf_und_stabilitaet` |
| B-17 | manipulierte Merge-Tabelle, Laden verweigert | `tests::b_17_manipulierte_merge_tabelle_wird_abgelehnt`, `tests::b_17_kaputte_kopfzeile_wird_abgelehnt` |

Dazu: SHA-256 gegen drei bekannte Vektoren, Basisvokabular gegen alle 256 Byte,
Tie-Break als eigene Zusicherung, typisierte Fehlerpfade (Ziel zu klein, Ziel zu
groß, Ziel unerreichbar, unbekanntes Token), Selbstladbarkeit der kanonischen Form
und Digest-Unterschied zwischen zwei Vokabularen.

## Bewusste Grenzen

- **Kein trainierter Korpus.** Es gibt in diesem Repository keine Trainingsdaten
  (D3/R2: Datenkontrolle ist offen). Die Tests trainieren auf einem festen
  Beispielkorpus im Testcode, nicht auf einem Zielkorpus.
- **Kein Vokabular-Digest im Modellartefakt.** `ATC-AI-LLM-002` bindet Gewichte und
  Numerik, nicht die Tokenizer-Identität. Das ist Punkt K-T1 im README.
- **Kein Sampler, kein KV-Cache.** Gehört zu P8/`atc-aurora-runtime`, nicht hierher.
- **Kein Parallelismus.** Ein Trainingslauf ist sequenziell und reproduzierbar; die
  Laufzeit ist für Zielkorpus-Größen noch nicht gemessen.

## Kosten

Der Testlauf mit dem 1-MB-Grenzfall dauert im Debug-Build etwa 44 Sekunden. Das ist
der Preis des Rundlaufs über 10 000 Folgen plus einen Megabyte Zufall, nicht des
Trainings.

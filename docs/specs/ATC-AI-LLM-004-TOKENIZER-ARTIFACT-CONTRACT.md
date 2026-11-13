---
document_id: ATC-AI-LLM-004
title: Kanonisches Tokenizer-Artefaktformat (K-T1) — normativer Vertrag
version: 1.0.0
status: proposed
owner: A-TownChain-Okosystems
created: 2026-11-13
updated: 2026-11-13
layer: L2 (aurora-ai)
security_class: S2
evidence_gate: AURORA-001
---

# Kanonisches Tokenizer-Artefaktformat (K-T1)

Dieser Vertrag legt fest, welche **Bytefolge** ein Aurora-Tokenizer-Artefakt
darstellt, in welcher Reihenfolge geprüft wird und welche Fehlerklassen es gibt.
Er ist die normative Autorität; die Implementierung in
`modules/atc-aurora-tokenizer` ist ihre Referenz, nicht ihr Ersatz.

## 1. Zweck und Abgrenzung

Ein Vokabular ist nur dann ein identifizierbares Artefakt, wenn zwei Läufe auf
identischer Quelle dieselben Bytes erzeugen und ein späterer Lauf diese Bytes
wieder als dasselbe Vokabular erkennt. Ohne diese Festlegung ist der Digest eine
Prüfsumme über eine Konvention, die niemand aufgeschrieben hat.

Nicht Gegenstand dieses Vertrags:

| Thema | zuständig |
|---|---|
| Modellgewichte und Numerik | `ATC-AI-LLM-002` (ATCM) |
| Herkunft und Signatur von Artefakten | `ATC-AI-LLM-003` (K7) |
| Herkunft, Reinigung und Zulässigkeit der Trainingsdaten | D3 / R2, offen |
| Trainingsalgorithmus, Zielvokabular, Korpus | `ATC-AI-LLM-001` §8 P1 |

Verbindliche Schlüsselwörter: **MUSS** = Abnahmebedingung, **DARF NICHT** =
Ausschluss, **SOLLTE** = begründete Abweichung zulässig.

## 2. Fassung

| Feld | Wert |
|---|---|
| Formatkennung | `ATC-BPE` |
| Fassung | `1` |
| Kodierung | ASCII, LF als Zeilenende |
| Digest | SHA-256 über die Datei-Bytes, 64 Zeichen Hex |
| Transport | Bytefolge, keine Kompression, kein Container, keine Signatur |

## 3. Allgemeine Gültigkeit

**C-01** Ein Artefakt ist **genau dann** gültig, wenn es C-02 bis C-24 vollständig
erfüllt. Es gibt keinen Zustand „teilweise gültig": ein Verstoß führt zur Abweisung
der ganzen Datei, nicht zu einem Teilobjekt.

**C-02** Die erste Zeile MUSS exakt `ATC-BPE 1` lauten. Jede andere Fassung ist
unbekannt und wird abgewiesen (E-01); sie wird **nicht** heuristisch gelesen.

**C-03** Die Datei MUSS ASCII sein: Bytes `0x20`–`0x7E` sowie `0x0A` als
Zeilentrenner. `0x0D` (CR), `0x09` (Tab), ein BOM und jedes Byte ≥ `0x80` sind
unzulässig (E-18). CRLF ist damit keine Variante, sondern ein Fehler.

## 4. Aufbau

**C-04** Die Zeilenfolge ist bindend:

```
ATC-BPE 1
vocab <n>
<id> <hexbytes>            × n
merges <m>
<links> <rechts> <neu>     × m
```

**C-05** Jede Zeile endet mit genau einem LF. Die **letzte** Zeile endet ebenfalls
mit LF. Fehlt der abschließende Zeilenumbruch, ist die Datei nicht kanonisch (E-16).

**C-06** Trennzeichen ist **genau ein** Leerzeichen (`0x20`). Doppelte Leerzeichen,
führende oder nachgestellte Leerzeichen und jedes andere Whitespace sind unzulässig.
Ein Escape-Mechanismus existiert nicht und wird nicht gebraucht: alle Felder sind
Dezimalzahlen oder Hex-Ziffern und enthalten damit kein Leerzeichen.

**C-07** Kommentare, Leerzeilen und zusätzliche Felder sind unzulässig. Überzählige
Zeilen nach dem letzten Merge werden abgewiesen (E-15).

## 5. Vokabularzeilen

**C-08** Aufbau: `<id> <hexbytes>`. `id` ist dezimal nach C-20.

**C-09** Die IDs laufen lückenlos und aufsteigend von `0` bis `n-1`. Jede andere
Reihenfolge oder Lücke wird abgewiesen (E-05). Eine Sortierung wird **nicht**
vorgenommen: die Datei ist die Ordnung.

**C-10** `hexbytes` sind die Bytes des Tokens als Hex, **kleingeschrieben**
(`0-9a-f`), mit gerader Stellenzahl, mindestens zwei Stellen. Großbuchstaben sind
unzulässig (E-07).

**C-11** Für `id < 256` MUSS das Token genau das Byte mit dem Wert `id` sein
(Basiskalibrierung). Ein Artefakt, dessen Token 65 nicht `41` ist, ist ungültig
(E-06), auch wenn es sonst konsistent wäre.

**C-12** Für `id ≥ 256` MUSS das Token die Konkatenation der Bytes von `links` und
`rechts` sein (E-13).

## 6. Merge-Zeilen

**C-13** Aufbau: `<links> <rechts> <neu>`, alle drei dezimal nach C-20.

**C-14** Die Anzahl `m` MUSS `n - 256` betragen (E-09). Ein Vokabular ohne
Merge-Zeilen ist nur als reines Byte-Vokabular (`n = 256`, `m = 0`) gültig.

**C-15** `neu` MUSS `256 + Zeilenindex` sein, Zeilenindex ab `0`. Die Datei ist
keine Menge, sondern eine geordnete Folge (E-11).

**C-16** `links` und `rechts` MÜSSEN auf zum Zeitpunkt der Zeile bereits definierte
Tokens verweisen, also `< 256 + Zeilenindex` (E-12). Vorwärtsverweise sind
unzulässig, auch wenn sie sich auflösen ließen.

**C-17** Ein Paar `(links, rechts)` DARF höchstens einmal vorkommen (E-14). Ein
zweites Vorkommen wäre ein zweiter Rang für dieselbe Paarung und macht die
Zerlegung mehrdeutig.

**C-18** Die Zeilenreihenfolge der Merges **ist** die Trainingsreihenfolge und damit
Teil der Identität: der Rang eines Paares entscheidet, welche Zerlegung entsteht.
Zwei Artefakte mit gleichen Paaren in anderer Reihenfolge sind verschiedene
Artefakte und haben verschiedene Digests.

**C-19** `n` MUSS im Bereich `256..=65536` liegen (E-03). 4096 ist der Zielwert aus
K4, nicht die Obergrenze des Formats.

## 7. Zahlenformate

**C-20** Ein Dezimalfeld ist entweder `0` oder `[1-9][0-9]*`. Unzulässig sind
führende Nullen (`0256`), Vorzeichen (`+256`, `-1`), Leerzeichen im Feld und leere
Felder (E-04/E-10). Der Wertebereich ist `0..=65535` (E-02/E-10).

## 8. Digest

**C-21** Der Digest ist SHA-256 (FIPS 180-4) über **exakt die Bytes der Datei**,
einschließlich aller LF. Es findet keine Vor- oder Nachbearbeitung statt: kein
Trimmen, kein Zeilenende-Umbau, keine Normalisierung, keine Auslassung der
Kopfzeile.

**C-22** Der Digest wird als **64 Zeichen kleingeschriebenes Hex** dargestellt.
Jede andere Form (Großbuchstaben, Kürzung, Präfix `sha256:`, Leerzeichen) wird
abgewiesen (E-17).

**C-23** Der Vergleich ist exakt und bitweise. Eine Normalisierung der Schreibweise
findet nicht statt; ein abweichender Digest ergibt `DigestMismatch` und damit keine
Verwendung des Artefakts.

## 9. Validierungsreihenfolge

**C-24** Geprüft wird in dieser Reihenfolge, und jeder Schritt ist fail-closed:

| # | Prüfung | Fehler |
|---|---|---|
| 1 | Digestformat (64 Zeichen, kleingeschriebenes Hex) | E-17 |
| 2 | Digest gegen die Datei-Bytes | `DigestMismatch` |
| 3 | Zeichensatz und Zeilenenden (ASCII, LF) | E-18 |
| 4 | Kopfzeile und Fassung | E-01 |
| 5 | `vocab`-Kopfzeile, Zahlenformat, Bereich | E-02, E-03 |
| 6 | Token-Zeilen: Aufbau, ID-Folge, Hex, Basisidentität | E-04, E-05, E-07, E-06 |
| 7 | `merges`-Kopfzeile und Anzahl | E-08, E-09 |
| 8 | Merge-Zeilen: Aufbau, ID-Folge, Verweis, Eindeutigkeit, Byte-Gleichheit | E-10, E-11, E-12, E-14, E-13 |
| 9 | Überzählige Zeilen | E-15 |
| 10 | Kanonizität durch Re-Serialisierung | E-16 |

Der Digest wird **vor** jedem Parsen geprüft: eine manipulierte Datei wird an
Schritt 2 abgewiesen, ohne dass ihre Struktur interpretiert wurde (B-17).

Schritt 10 ist das Sicherheitsnetz: nach dem Parsen wird der Zustand erneut
serialisiert und byteweise mit der Eingabe verglichen. Damit ist jede Abweichung
erfasst, die die Einzelprüfungen nicht benennen, und die Import-Route kann
strukturell niemals mehr akzeptieren als die Export-Route erzeugt.

## 10. Fehlerklassen

| Klasse | Bedeutung |
|---|---|
| `E-01` | Kopfzeile oder Fassung unbekannt |
| `E-02` | `vocab`-Kopfzeile fehlerhaft |
| `E-03` | Vokabulargröße außerhalb `256..=65536` |
| `E-04` | Token-Zeile fehlerhaft (Aufbau oder Zahlenformat) |
| `E-05` | Token-IDs nicht lückenlos aufsteigend |
| `E-06` | Basisidentität verletzt (Token `id < 256` nicht das Byte `id`) |
| `E-07` | Hex-Feld unzulässig (Zeichen, Stellenzahl) |
| `E-08` | `merges`-Kopfzeile fehlerhaft |
| `E-09` | Merge-Anzahl passt nicht zum Vokabular |
| `E-10` | Merge-Zeile fehlerhaft (Aufbau oder Wertebereich) |
| `E-11` | Merge-IDs nicht fortlaufend ab 256 |
| `E-12` | Merge verweist auf ein nicht definiertes Token |
| `E-13` | Merge-Bytes stimmen nicht mit der Konkatenation überein |
| `E-14` | Paar doppelt vorhanden |
| `E-15` | Überzählige Zeilen |
| `E-16` | Datei nicht kanonisch (Re-Serialisierung weicht ab) |
| `E-17` | Digestformat unzulässig |
| `E-18` | Zeichensatz oder Zeilenende unzulässig |
| `DigestMismatch` | Digest passt nicht zu den Bytes |

Alle Klassen sind terminal. Es gibt keine Warnung ohne Abbruch und kein
Reparaturverhalten beim Import.

## 11. Kompatibilität und Versionierung

**C-25** Die Fassung `1` ist mit diesem Dokument eingefroren. Jede Änderung an
Zeichenvorrat, Feldreihenfolge, Zahlenformat, Digest-Eingabe oder
Validierungsreihenfolge erfordert eine neue Fassung mit eigener Klauselmenge. Eine
Fassung wird nicht stillschweigend erweitert.

**C-26** Ein Leser MUSS unbekannte Fassungen abweisen (C-02) und DARF sie nicht
näherungsweise interpretieren. Ein Vokabular wird erst dann eingefroren, wenn ein
Artefakt nach dieser Fassung erzeugt, geprüft und mit dem Digest referenziert ist.

## 12. Konformitätsmatrix der Referenzimplementierung

Stand 13.11.2026, `modules/atc-aurora-tokenizer`, HEAD `29da156` plus P1-B-Commit.
„PASS" heißt: die Klausel wird durch Code und benannten Test abgedeckt.

| Klausel | Umsetzung | Prüfung | Stand |
|---|---|---|---|
| C-01 | `from_canonical` liefert `Result`, kein Teilobjekt | `kanonische_form_laedt_sich_selbst` | PASS |
| C-02 | Kopfzeilenvergleich auf `ATC-BPE 1` | `b_17_kaputte_kopfzeile_wird_abgelehnt` | PASS |
| C-03 | ASCII- und CR-Prüfung vor dem Parsen | `c_03_zeichensatz_und_cr_werden_abgewiesen` | PASS |
| C-04 | feste Zeilenfolge im Parser | `kanonische_form_laedt_sich_selbst` | PASS |
| C-05 | abschließendes LF durch Kanonizitätsprüfung | `c_05_fehlendes_abschliessendes_lf_ist_nicht_kanonisch` | PASS |
| C-06 | genau ein Leerzeichen, `split_once`/drei Felder | `c_06_doppeltes_leerzeichen_wird_abgewiesen` | PASS |
| C-07 | überzählige Zeilen | `c_07_ueberzaehlige_zeile_wird_abgewiesen` | PASS |
| C-08 | Aufbau Token-Zeile | `b_17_manipulierte_merge_tabelle_wird_abgelehnt` | PASS |
| C-09 | ID-Folge 0..n-1 | `c_09_token_ids_muessen_fortlaufend_sein` | PASS |
| C-10 | Hex kleingeschrieben, gerade Stellenzahl | `c_10_grossbuchstaben_im_hexfeld_werden_abgewiesen` | PASS |
| C-11 | Basisidentität `tokens[i] == [i]` | `c_11_basisidentitaet_wird_geprueft` | PASS |
| C-12 | Byte-Konkatenation je Merge | `b_17_manipulierte_merge_tabelle_wird_abgelehnt` | PASS |
| C-13 | drei Dezimalfelder | `c_13_merge_zeile_mit_zwei_feldern_wird_abgewiesen` | PASS |
| C-14 | `m == n - 256` | `c_14_merge_anzahl_passt_nicht_zum_vokabular` | PASS |
| C-15 | `neu == 256 + Index` | `c_15_merge_ids_muessen_fortlaufend_sein` | PASS |
| C-16 | Vorwärtsverweise abgewiesen | `c_16_vorwaertsverweis_wird_abgewiesen` | PASS |
| C-17 | Eindeutigkeit der Paare | `c_17_doppeltes_paar_wird_abgewiesen` | PASS |
| C-18 | Reihenfolge ist Identität, Digest hängt daran | `c_18_reihenfolge_aendert_den_digest` | PASS |
| C-19 | Bereich `256..=65536` | `training_bricht_typisiert_ab_wenn_das_ziel_nicht_erreichbar_ist` | PASS |
| C-20 | strikte Dezimalfelder | `c_20_fuehrende_null_wird_abgewiesen` | PASS |
| C-21 | Digest über die Datei-Bytes | `kanonische_form_laedt_sich_selbst` | PASS |
| C-22 | 64 Zeichen, kleingeschriebenes Hex | `c_22_digestformat_wird_geprueft` | PASS |
| C-23 | exakter Vergleich, keine Normalisierung | `c_23_digest_vergleich_ist_exakt` | PASS |
| C-24 | Reihenfolge der Prüfungen, fail-closed | `b_17_manipulierte_merge_tabelle_wird_abgelehnt`, `c_24_digest_vor_struktur` | PASS |
| C-25 | Fassung 1 eingefroren | dieses Dokument | PASS |
| C-26 | unbekannte Fassung abgewiesen | `b_17_kaputte_kopfzeile_wird_abgelehnt` | PASS |

Grenzen dieser Matrix: sie belegt Klauselabdeckung, nicht CI-Ausführung. Ein
Rust-CI-Lauf existiert nicht (F-T2), und es gibt keinen Zielkorpus (D3/R2), gegen
den ein Vokabular hätte geprüft werden können.

## 13. Offene Punkte

| Punkt | Stand |
|---|---|
| Vokabular einfrieren | erst nach P1-E (Benchmark auf dem P1-Zielkorpus) |
| Digest-Autorität | F-T1: zwei SHA-256-Umsetzungen im Repo, Entscheidung offen |
| Rust-CI | F-T2: Workflow ohne Rust-Job |

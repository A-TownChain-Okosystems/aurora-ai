---
document_id: ATC-AI-MEAS-002
title: Messung — Kosten von Lesen, Hashen und Dekodieren eines ATCM-Artefakts (Ref-1)
version: 1.0.0
status: accepted
owner: A-TownChain-Okosystems
created: 2026-11-11
updated: 2026-11-11
subject: ATC-AI-LLM-002 v1.2.0, aurora-ai HEAD 6bcd1a524
layer: L2 (aurora-ai)
evidence_gate: AURORA-001
---

# Messung: Lesen, Hashen, Dekodieren

**Anlass.** Der Owner hat als Folgeprüfung verlangt, „Speicherverbrauch und
Dekodierkosten von `tensor_f32` gegenüber `to_weights()`" zu messen. Diese Notiz
ist die Antwort, mit Methode und Zahl.

## 1. Aufbau

- Referenzkonfiguration `Config::ref1()` (d_model 256, 6 Blöcke, Vokabular 4096,
  `d_ff` 704, gebundene Einbettung) und deterministische Gewichte aus Startwert 1.
- Eine Datei, `23478504` Byte (23,5 MB), 50 Tensoren, 5 868 800 Elemente.
- Messung als `#[ignore]`-Test im Crate, damit sie reproduzierbar bleibt:
  `cargo test --release -- --ignored --nocapture messung` in
  `modules/atc-aurora-model`. Release-Profil, ein Durchlauf, keine Wiederholung.

## 2. Ergebnis

| Schritt | Zeit |
|---|---|
| `read` (Struktur, Tabelle, Digest, Anker) | **65 ms** |
| davon SHA-256 über die ganze Datei allein | **65 ms** |
| `tensor_f32` für alle 50 Tensoren (Summe) | **5,6 ms** |
| `to_weights()` (eine Vollkopie) | **13,4 ms** |

Durchsatz: 23,5 MB in 65 ms sind rund **360 MB/s** für die Digest-Berechnung.

## 3. Was die Zahlen bedeuten

**Der Digest ist der Preis, nicht die Dekodierung.** Strukturprüfung und Tabelle
kosten zusammen unter einer Millisekunde; praktisch die gesamte Lesezeit ist
SHA-256 über die Datei. Das ist die messbare Begründung für die Regel aus
`ATC-AI-LLM-002` Abschnitt 8.1 Punkt 1: `max_bytes` muss **vor** dem Hashen
greifen, weil sonst schon das Prüfen einer Datei proportional zu ihrer Größe
fremde Rechenzeit verbraucht.

**Dekodierung ist billig, aber nicht beliebig oft.** Ein Tensor kostet im
Durchschnitt 0,11 ms; alle 50 zusammen 5,6 ms. Wer im Vorwärtspass bei jedem
Aufruf dekodiert, zahlt das je Aufruf. Der übliche Weg ist `to_weights()` einmal
zu Beginn.

**Die Vollkopie ist nur 2,4-mal teurer als das Dekodieren.** Der Aufpreis ist die
Tensor-Konstruktion mit Formprüfung je Tensor, nicht das Kopieren der Bytes. Das
ist günstig genug, dass die Nullkopie-Frage keine Sicherheitsabstriche rechtfertigt:
`to_weights()` bleibt der Standardweg, `tensor_f32` der Weg für einzelne Tensoren.

## 4. Speicherverbrauch

| Weg | Zusätzlicher Speicher |
|---|---|
| `read` | Datei-Bytes (Aufrufer) + 50 Beschreibungen à 64 Byte, also rund 3 KB |
| `tensor_f32(i)` | ein Tensor, `4 · Elemente` Byte, freigegeben nach dem Aufruf |
| `to_weights()` | 23,5 MB, dauerhaft bis zum Freigeben der Gewichte |

Die Zusage aus Abschnitt 8.1 Regel 2 hält damit messbar: der Lesevorgang
allokiert nicht proportional zum Modell.

## 5. Folgen für K7 und K8

- **K8 (Kompression):** da der Digest die Lesezeit dominiert, verkürzt eine
  komprimierte Datei die Lesezeit nahezu proportional zum Kompressionsverhältnis.
  Der Preis wäre eine zweite Digest-Ebene (über die komprimierten Bytes und über
  die entpackten), und Entpacken bringt genau die Größenrisiken zurück, die
  Abschnitt 8 gerade ausschließt. K8 bleibt deshalb zu Recht getrennt.
- **K7 (Signatur):** die Signaturprüfung würde über den 32-Byte-Digest laufen und
  ist damit im Vergleich zu 65 ms Hashing nicht messbar. Die Kosten von K7 liegen
  nicht in der Rechenzeit, sondern in der Schlüsselverwaltung.
- **Naheliegende Optimierung, nicht beschlossen:** ein Zwischenspeicher für den
  Digest bei unveränderten Dateien (Größe plus Änderungszeit) würde wiederholtes
  Lesen fast kostenlos machen. Das ist eine Aufruferfrage, keine Formatfrage, und
  gehört nicht in den Vertrag.

## 6. Grenzen der Messung

Ein Lauf, ein Rechner, ein Release-Build; keine Aussage über andere Hardware und
keine Streuungsanalyse. Die Zahlen taugen als Größenordnung und als Verhältnis,
nicht als Garantie. Wer sie als Zusage braucht, muss sie mit Wiederholungen und
auf der Zielplattform erheben.

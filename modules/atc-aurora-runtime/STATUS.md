# 📊 Status — atc-aurora-runtime

> **Stand:** 2026-11-10 (korrigiert)
> **Korrekturhinweis:** Die vorherige Fassung vom 2026-08-08 war in zwei Punkten
> falsch und ist ersetzt. Sie behauptete "0 `.atc` Dateien (geplant: 5)", während
> fünf Dateien vorhanden sind, und sie verschwieg, dass dieses Modul weder ein
> `Cargo.toml` noch eine Rust-Quelldatei besitzt. Beide Aussagen sind unten
> richtiggestellt.

## Gesamt-Status

| Metrik | Wert |
|--------|------|
| Rust-Quellen (`*.rs`) | 0 |
| `Cargo.toml` | nicht vorhanden |
| `.atc` Dateien | 5 (Spezifikationsnotizen, je etwa 950 Byte) |
| Ausführbarer Code | keiner |
| Tests | 0 |
| Status Code | `MISSING` |
| Status Notizen | `ARCHITECTURE_ONLY` |

Dieses Modul ist heute ein Platzhalter. Die fünf `.atc`-Dateien sind Notizen zur
geplanten Schnittstelle, kein ausführbarer Code. Es gibt in diesem Repository
keinen ATCLang-Parser-Lauf und keinen CI-Schritt, der sie prüft; ob sie mit einem
ATCLang-Parser parsen, ist deshalb **nicht nachgewiesen**. Nach
`ATC-DOC-AI-SSOT-004` ist das Verzeichnis als geplante ATCLang-Schnittstelle
eingeordnet.

## Komponenten-Status

| Komponente | Datei vorhanden | Ausführbar | Geprüft |
|-----------|-----------------|------------|---------|
| `src/agent_runtime.atc` | ja | nein | nein |
| `src/tool_executor.atc` | ja | nein | nein |
| `src/sandbox.atc` | ja | nein | nein |
| `src/task_scheduler.atc` | ja | nein | nein |
| `src/result_collector.atc` | ja | nein | nein |

Die funktionalen Gegenstücke, die tatsächlich laufen, liegen in
`modules/atc-aurora-core` als Rust: Aufgabenplanung, Ergebnis-Sammlung und
Sandbox-Grenzen werden dort implementiert, nicht hier. Ein zweiter Scheduler in
diesem Modul wäre ein Verstoß gegen `ATC-DOC-AI-SSOT-004`.

## Nächster Schritt

Die Inferenz-Aufgabe aus `ATC-AI-LLM-001` (Phase P8) wird dieses Modul erst
brauchen, wenn Tensor-Engine, Tokenizer, Transformer und Training stehen. Bis
dahin bleibt es unverändert ein Platzhalter, und es wird kein Code hier
angelegt.

# 📝 Changelog — atc-aurora-memory

## [Unreleased] — 2026-10-17

### Hinzugefügt
- Rust-MVP `memory_engine.rs` für sitzungsgebundenes Kurzzeitgedächtnis und freigegebene Langzeitgedächtniseinträge.
- Mandanten-, Benutzer-, Agenten- und Session-Scope sowie begrenzte Inhalte, Ablaufzeiten, Provenienz und Vertrauensstatus.
- Context Builder und lesbare Explanation Records mit Quellenreferenzen, Entscheidungskurzfassung und Unsicherheiten.
- Fail-closed `DerivedArtifactCleaner`-Vertrag: kanonischer Eintrag wird bei fehlgeschlagener Bereinigung nicht gelöscht.
- Unit-Tests für Kontextbegrenzung, Scope-Isolation, Freigabe/Trust, Ablauf, Secret-Marker, Löschung und Quellenbindung.

- Versionierter, dateibasierter Journal-Adapter mit Begrenzungen, fail-closed Parsing, Scope-Prüfung sowie Restart-/Delete-/Migrations-Tests.
- `MemoryManager::open_persistent` bindet den Journal-Adapter für freigegebene Langzeitdaten ein; Manager-Integrationstests prüfen Wiederherstellung, ID-Fortschreibung und Löschung über Neustarts.

### Einschränkungen / offen
- `MemoryManager` hält weiterhin eine In-Memory-Arbeitskopie; bei `open_persistent` werden Langzeitdatensätze aus dem Journal wiederhergestellt und Schreib-/Löschoperationen an das Journal weitergereicht.
- `MemoryIndex` und `VectorStore` bleiben getrennte bestehende Module; ihre produktive Persistenz und koordinierte Index-/Embedding-/Cache-Bereinigung sind nicht durch diese Änderung belegt.
- Keine Verschlüsselung im Ruhezustand; robuste Secret-Erkennung und GlobusOS-Capability-Integration bleiben offen.
- Robuste Secret-Erkennung, produktive Index-/Embedding-/Cache-Bereinigung und GlobusOS-Capability-Integration bleiben offen.
- CI-Evidenz gilt ausschließlich für den vollständigen Commit-SHA, für den die Workflows ausgeführt wurden; nach Änderungen ist der neue Head erneut zu prüfen.

---
*Maintained manually; implementation status is bound to exact source SHA and CI evidence.*

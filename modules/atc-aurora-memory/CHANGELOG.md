# 📝 Changelog — atc-aurora-memory

## [Unreleased] — 2026-10-17

### Hinzugefügt
- Rust-MVP `memory_engine.rs` für sitzungsgebundenes Kurzzeitgedächtnis und freigegebene Langzeitgedächtniseinträge.
- Mandanten-, Benutzer-, Agenten- und Session-Scope sowie begrenzte Inhalte, Ablaufzeiten, Provenienz und Vertrauensstatus.
- Context Builder und lesbare Explanation Records mit Quellenreferenzen, Entscheidungskurzfassung und Unsicherheiten.
- Fail-closed `DerivedArtifactCleaner`-Vertrag: kanonischer Eintrag wird bei fehlgeschlagener Bereinigung nicht gelöscht.
- Unit-Tests für Kontextbegrenzung, Scope-Isolation, Freigabe/Trust, Ablauf, Secret-Marker, Löschung und Quellenbindung.

- Versionierter, dateibasierter Journal-Adapter mit Begrenzungen, fail-closed Parsing, Scope-Prüfung, Restart-/Delete-/Migrations-Tests; noch nicht in `MemoryManager` verdrahtet.

### Einschränkungen / offen
- Der bestehende `MemoryManager` bleibt In-Memory; der neue Journal-Adapter ist ein separater Persistenzbaustein und noch nicht in den Manager-/Index-/Embedding-Löschpfad integriert.
- Keine Verschlüsselung im Ruhezustand; robuste Secret-Erkennung und GlobusOS-Capability-Integration bleiben offen.
- Robuster Secret-Scanner, persistenter Storage-Adapter, produktive Index-/Embedding-Bereinigung und GlobusOS-Capability-Integration bleiben offen.
- CI Build/Test war für den vorherigen PR-Head erfolgreich; neuer Head nach Changelog-Änderung muss erneut geprüft werden.

---
*Maintained manually; implementation status is bound to exact source SHA and CI evidence.*

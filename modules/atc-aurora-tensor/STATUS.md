# STATUS: atc-aurora-tensor

| Bereich | Status | Beleg |
|---|---|---|
| M-03 Implementierung | `IMPLEMENTED` | Rechengraph, Rückwärtspass, Akkumulation, transaktionaler Fehlerpfad |
| Frischer Klon / Tests | `TESTED` | 53 Tests grün aus frischem Klon am 11.11.2026 |
| Lokale technische Prüfung | nachgewiesen | clippy 0 Warnungen, rustfmt sauber, Bare-Metal-Build grün, Register-Invariante Exit 0 |
| `CI_VERIFIED` | **offen** | alle Workflow-Läufe `startup_failure` ohne Job, siehe Abschnitt Nachweise |
| `E2E_VERIFIED` | **offen** | kein Lauf im Gesamtsystem; M-05/M-06 existieren noch nicht |
| Mirror-Integration | `IN_SYNC` | `sync_modules.py --check` aus frischem Mirror-Klon: Exit 0, 437 von 437 Scope-Dateien byte-identisch |
| K6 (LayerNorm, GELU, Dropout, Embedding) | `RESIDUAL` | bewusst nicht in M-03, eigene Contract-Prüfung nötig |
| Git-Identität | `BLOCKED` | E-Mail des Kontos unbestätigt, `git push` 403, Git-Data-API 403 |
| Performance-Referenz (T2.6) | `BLOCKED_EXTERNAL` | keine qualifizierte Maschine (K3) |

`CI_VERIFIED` wird ausdrücklich nicht beansprucht: push-getriggerte Workflows
scheitern derzeit am unbestätigten GitHub-Konto. `TESTED` heißt hier lokal
ausgeführt, nicht in CI bestätigt.

## P2-Bedingungen nach Blueprint Kapitel 4

| Bedingung | Erfüllt | Beleg |
|---|---|---|
| `P2_IMPLEMENTATION_COMPLETE` | ja | M-01 bis M-03 im Workspace baubar, `cargo build` und Bare-Metal-`cargo check --target thumbv7em-none-eabihf` grün |
| `P2_FUNCTIONALLY_TESTED` | ja | B-01 bis B-13 grün, 53 Tests aus frischem Klon |
| `P2_PERFORMANCE_VERIFIED` | nein | T2.6 `BLOCKED_EXTERNAL`, keine qualifizierte Maschine (K3) |
| `P2_ACCEPTED` | nein | setzt alle drei vorgenannten voraus |

## Testfälle aus P2

| Fall | Prüfung | Status |
|---|---|---|
| B-01 | Matmul mit unpassenden Formen | grün (`b_01_*`) |
| B-02 | Addition nicht kompatibler Formen | grün (`b_02_*`) |
| B-03 | Budgetüberschreitung vor der Zuteilung | grün (`b_03_*`) |
| B-04 | Konstruktion aus Little-Endian-Bytes | grün (`b_04_*`) |
| B-05 | Matmul gegen handgerechnetes 2x2-Beispiel | grün (`b_05_*`, zwei Tests) |
| B-06 | Doppelte Transponierung und Reihenfolgeidentität | grün (`b_06_*`, zwei Tests) |
| B-07 | Softmax mit Logits ±1e5 | grün (`b_07_*`, zwei Tests) |
| B-08 | Kreuzentropie, Gradient exakt `softmax − target`, Zielwahrscheinlichkeit 0 | grün (`b_08_*`, zwei Tests) |
| B-09 | Matmul-Gradient gegen zentrale Finitedifferenz | grün (`b_09_*`) |
| B-10 | Kette Softmax und Kreuzentropie | grün (`b_10_*`, zwei Tests) |
| B-11 | RMSNorm, `ε = 1e-8`, normnahe und exakt null Zeile | grün (`b_11_*`, drei Tests) |
| B-12 | Zwei Rückwärtspässe auf dasselbe Blatt, Raute | grün (`b_12_*`, zwei Tests) |
| B-13 | Zusammengesetztes Netz, zwei Schichten, alle Parameter | grün (`b_13_*`) |

B-08 bis B-13 sind aus dem Contract abgeleitet, nicht aus dem Verhalten einer
Vorlage: die Vorlage hatte keinen Rechengraphen. Referenz ist überall die zentrale
Finitedifferenz in f64 mit `h = 1e-5`, gebildet im Test.

## Messwerte gegen die Toleranzen

| Prüfung | Grenze | gemessen |
|---|---|---|
| B-09 dA / dB | 1e-6 | 4,6e-8 / 4,1e-8 |
| B-10 Kette / Softmax-JVP | 1e-6 | 1,3e-7 / 1,8e-7 |
| B-11 dx / dw | 1e-5 | 1,6e-6 / 2,4e-7 |
| B-13 w0 / W1 / W2 | 1e-3 (f32-Pfad) | 4,6e-8 / 5,0e-6 / 4,3e-7 |

B-13 liegt gemessen auch unter der strengeren 1e-4-Grenze, die der Contract für
einen f64-Pfad nennt; geprüft wird die f32-Bedingung, weil das Crate f32 als
einzigen Puffer führt.

## Wirksamkeit der Autograd-Tests

Fünf Mutationen am Rückwärtspfad, jede von den erwarteten Tests erkannt:

| Mutation | erkannt von |
|---|---|
| `matmul`-Gradient ohne Transponierung des zweiten Faktors | B-09, B-12, B-13 |
| Softmax-Gradient aus dem Eingang statt aus dem Ausgang | B-10 Softmax-JVP |
| Kreuzentropie-Gradient mit `1/n` statt Summe | B-08, B-10 |
| RMSNorm ohne den zweiten Term der Ableitung | B-11 |
| Akkumulation in der Verteilung überschreibt statt addiert | B-12 zwei Pässe **und** B-12 Raute |
| Akkumulation im Parameterspeicher überschreibt statt addiert | B-12 zwei Pässe |

**Lücke geschlossen (11.11.2026).** Bis zum 11.11. erkannte nur die Raute die
Akkumulations-Mutation, nicht der Test der zwei Rückwärtspässe. Der Test prüft
jetzt beide Akkumulationsstellen in einem Fall: eine Raute (zweiter Beitrag auf
denselben Parameter innerhalb eines Passes) **und** zwei Rückwärtspässe. Die
Erwartungswerte sind von Hand gerechnet und dyadisch exakt: `W1 = [[1,2],[3,4]]`
ergibt Zeilensummen `[3,7]`, `W2 = [[0.5,-0.5],[2,1]]` ergibt `[0,3]`, also ein
Pass `[3,10]`, zwei Pässe `[6,20]`, drei Pässe `[9,30]`. Beide Mutationen lassen
genau diesen Test fehlschlagen, die Verteilungs-Mutation zusätzlich die Raute.
Die Asymmetrie ist damit erklärt und nicht mehr offen: die Raute kann die
Addition über Pässe nicht sehen, weil sie nur einen Pass fährt.

Drei Mutationen am Operationssatz aus Phase A bleiben gültig (Matmul ohne letzten
Summanden, Softmax ohne Maximum-Subtraktion, Transponierung ohne Indexvertauschung).

## Zuständigkeitsgrenze

`abhaengigkeitsregel_nur_libm` liest das eigene `Cargo.toml` und prüft bei jedem
Testlauf: als Abhängigkeit ist ausschließlich `libm` zulässig, und kein
Kernel-Crate, keine zweite Tensor-Bibliothek (torch, candle, burn) darf im
Manifest auftauchen. Damit ist die Grenze aus AD-026 und AD-028 nicht nur
dokumentiert, sondern bricht den Testlauf, wenn sie verletzt wird.

## Vertrag der Autograd-Seite

`AUTOGRAD-CONTRACT.md` hält fest, welche zehn Operationen eine Ableitung haben und
warum die übrigen keine bekommen, welche Ableitung verbindlich ist, welche
Speicherlebensdauer der Graph verlangt und wo der Contract schweigt (Festlegungen
F1 bis F5).

## Integrations- und Evidenzphase (11.11.2026)

| Prüfung | Ergebnis |
|---|---|
| Kanon `aurora-ai` | Stand dieses Commits |
| Mirror `a-townchain-ecosystem` | IN_SYNC; **keine Nummer gepinnt**, weil jeder Sync den Mirror-HEAD weiterzieht — der Nachweis ist der Blob-Vergleich, nicht eine Nummer |
| Mirror-Sync über `scripts/sync_modules.py` | `--check` vorher: `diff=2 fehlt=15 orphan=0`; `--sync` kopiert 17 Dateien |
| Mirror `--check` aus frischem Klon | `IN_SYNC`, 1/1 Komponenten, Exit 0 |
| Modulvollständigkeit | `components/aurora-ai/modules/atc-aurora-tensor` vorhanden, 14 Dateien |
| Blob-Vergleich je Datei | 437 von 437 Scope-Dateien identisch, 0 abweichend (Vergleich der Blob-Hashes aus dem committeten Stand beider Repos) |
| Crate-Dateien im engeren Sinn | 15 von 15 identisch (14 Crate-Dateien + `tools/check_file_register.py`) |
| Tests auf dem gespiegelten Stand | `cargo test` auf den Mirror-Bytes: 53 grün |
| Keine zweite Aurora-Implementierung im Mirror | die gespiegelten Dateien sind byte-identisch, keine eigene Fassung, keine Orphans |

Der Sync lief ausschließlich über den vorgesehenen Mechanismus; keine Datei wurde
von Hand kopiert. Die Übertragung in das Mirror-Repository erfolgte über die
Contents-API, weil `git push` am unbestätigten Konto scheitert (siehe unten).

## CI-Lage (11.11.2026, reproduziert)

| Beobachtung | Ergebnis |
|---|---|
| `workflow_dispatch` auf `code-quality.yml` und `test-suite.yml` | beide gestartet, beide `startup_failure` |
| Jobs je Lauf | **keine** (`jobs: []`), keine Logs, keine Artefakte |
| Check-Runs zum Commit | keine |
| Repo-Zustand | Actions `enabled: true`, `allowed_actions: all`, beide Workflows `state: active` |
| Push-getriggerte Läufe | ebenfalls `startup_failure` |

Damit ist belegt: die Ursache liegt **nicht** am Trigger und **nicht** am
Workflow-Inhalt; die Läufe kommen über den Startpunkt nicht hinaus. Bis das
Konto verifiziert ist, gibt es keine CI-Evidenz, und ein grüner lokaler Build
ersetzt sie nicht.

## Offen

- **K6 `RESIDUAL`**: LayerNorm, GELU, Dropout und Einbettungs-Lookup nennt der
  P2-Absatz, die API-Fläche und die 24 Testfälle nennen sie nicht. Owner-Entscheidung
  vom 11.11.2026: **nicht** pauschal in M-03 nachziehen, sondern je Operator einen
  eigenen Contract prüfen (Definition, Vorwärts-API, Rückwärts-API, Speicher,
  Determinismus, Tests, Mutationsnachweis). Dropout braucht zusätzlich eine
  Randomness- und Reproduzierbarkeitsstrategie. Einzelheiten in
  `AUTOGRAD-CONTRACT.md` Abschnitt 7.
- **CI und Git-Identität `BLOCKED`**: nur der Owner kann die E-Mail-Adresse
  bestätigen. Ohne sie kein regulärer Push und kein Workflow-Start.
- **M-06**: Trainingsschleife mit Mittelwertbildung baut auf dem M-03-Contract auf;
  die Reduktionsstrategie bleibt dort, nicht im Operator.
- **T2.6 / K3**: Durchsatzmessung braucht die qualifizierte Maschine.
- SIMD und Ausrichtung: bewusst nicht Teil von P2. Der Kern bleibt die einzige
  semantische Implementierung.

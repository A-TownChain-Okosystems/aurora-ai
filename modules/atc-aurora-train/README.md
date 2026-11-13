# atc-aurora-train

M-06 `train` der Aurora-Referenzimplementierung: die Trainings-Engine, die aus
dem Vorwaertslauf von M-05 einen lernenden Lauf macht. Dieses Crate schliesst die
Modellkette ab dem Optimierer:

```text
Tokenizer -> Autograd -> Backpropagation -> Optimizer -> Training
          -> Evaluation -> Registry -> Inferenz
```

## Was hier liegt

| Baustein | Typ | Aufgabe |
|---|---|---|
| `Gradients` | Speicher | Beitraege je Sequenz einsammeln, in kanonischer Parameterfolge |
| `HyperParams` | Konfiguration | Lernrate, Momente, Daempfung, Begrenzung, Budget |
| `Adam` | Optimierer | Adam mit entkoppelter Gewichtsdaempfung (AdamW) |
| `Trainer` | Schleife | Graph je Sequenz, Rueckwaertspass, Mittelung, Schritt, Buchhaltung |
| `evaluate` | Bewertung | mittlerer Verlust, Perplexitaet, Trefferquote |
| `Checkpoint` / `Registry` | Pruefpunkte | Modell als ATCM plus Zustand, geordnet nach Schritt |

## Was hier **nicht** liegt

Keine Tensor-, Norm-, Attention- oder Hash-Implementierung. Der Vorwaertslauf
kommt aus `atc-aurora-model`, die Ableitung aus `atc-aurora-tensor`, die
Pruefsumme aus `atc-aurora-digest`. Ein Test prueft, dass die Gewichte eines
Pruefpunkts bytegleich zurueckkommen; ein zweiter, dass ein fortgesetzter Lauf
denselben naechsten Schritt ergibt wie der unterbrochene.

## Benutzung

```rust
use atc_aurora_model::config::Config;
use atc_aurora_model::weights::Weights;
use atc_aurora_train::{HyperParams, Trainer};

let cfg = Config::ref1();
let weights = Weights::from_seed(&cfg, 1);
let mut trainer = Trainer::new(cfg, weights, HyperParams::adam(3e-4))?;

// Ein Schritt = ein Stapel; Mittelung ueber alle Zielpositionen.
let report = trainer.step(&[vec![1, 2, 3, 4]])?;

// Pruefpunkt: Modell plus Trainingszustand.
let checkpoint = trainer.checkpoint()?;
let mut registry = atc_aurora_train::Registry::new();
registry.register(checkpoint)?;

// Fortsetzen: derselbe Lauf, nicht ein neuer mit denselben Gewichten.
let mut resumed = Trainer::from_checkpoint(cfg, registry.latest().unwrap(), 1 << 28)?;
```

## Vertragliche Festlegungen

1. **Mittelung.** `cross_entropy` summiert ueber die Zeilen (B-08); die Division
   durch die Zahl der Zielpositionen passiert im Graphen, nicht nachtraeglich an
   den Gradienten. Damit haengt der Verlust nicht von der Stapelgroesse ab.
2. **Ein Graph je Sequenz.** Kausale Attention darf nicht ueber Sequenzgrenzen
   sehen; ein gemeinsamer Graph mit Auffuellen waere eine andere Rechnung.
3. **Kein Zufall.** Kein Mischen, kein Dropout, kein Zufallsstrom. Derselbe
   Korpus mit denselben Gewichten ergibt auf jeder Maschine denselben Verlauf.
4. **Pruefpunkt = zwei Teile.** Modell als ATCM mit Pruefsumme als externer
   Anker, Zustand als `AURST001` mit eigener Pruefsumme. Ein veraenderter Teil
   wird abgelehnt, nicht repariert.
5. **AdamW.** Gewichtsdaempfung entkoppelt vom Gradienten (`w - lr*wd*w`), weil
   eine L2-Strafe im Gradienten mit der Momentenschaetzung wechselwirkt.

## Grenzen (bewusst offen)

- Kein KV-Cache, kein Batching im Vorwaertslauf, kein Sampler ausser `argmax`
  (`atc-aurora-model::decode`). Diese Teile gehoeren zu P8/`atc-aurora-runtime`.
- Kein Dateisystem: das Register haelt Pruefpunkte im Speicher, die Ablage
  entscheidet der Aufrufer.
- Keine Lernratenplanung, kein Early Stopping, keine Datenaufbereitung. Diese
  Schichten sind nicht Teil des Modellvertrags.

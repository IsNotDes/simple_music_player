# simple_music_player

Un lecteur audio CLI entièrement écrit en Rust, avec une interface TUI (terminal user interface) construite avec **ratatui** et un moteur audio basé sur **rodio**.

L'architecture repose sur plusieurs threads séparés : un thread dédié au décodage audio, un thread pour l'analyse FFT temps réel, et le thread principal pour l'UI — le tout synchronisé via `Arc` et des ring buffers.

---

## Fonctionnalités

- Lecture de fichiers **MP3, FLAC et WAV** (récursive dans le dossier `music/`)
- Interface TUI complète : liste de lecture, barre de recherche, statut de lecture
- **Visualiseur de spectre audio en temps réel** (FFT 1024 points avec fenêtre de Hanning)
- Contrôles clavier : lecture/pause, piste suivante/précédente, seek ±5s
- Recherche dans la playlist en temps direct
- Multi-threading propre avec arrêt gracieux des threads audio

---

## Architecture

```
┌─────────────────────────────────────┐
│           Thread principal (UI)     │
│   ratatui + crossterm @ 60 fps      │
└────────────────┬────────────────────┘
                 │ Arc>>
     ┌───────────┴───────────┐
     │                       │
┌────▼──────────┐   ┌────────▼────────┐
│ Thread audio  │   │  Thread FFT     │
│ (décodage +   │   │  (analyse       │
│  ring buffer) │   │   spectrogram)  │
└───────────────┘   └─────────────────┘
```

Le thread audio pousse des `f32` samples dans deux ring buffers séparés : un vers `rodio` pour la lecture, un vers le thread FFT pour l'analyse. Cela évite tout blocage entre UI et audio.

---

## Installation

```bash
git clone https://github.com/IsNotDes/simple_music_player
cd simple_music_player
cargo build --release
```

Placez vos fichiers audio dans le dossier `music/` (sous-dossiers supportés).

```bash
cargo run --release
```

---

## Contrôles

| Touche       | Action                           |
|--------------|----------------------------------|
| `Espace`     | Lancer la piste sélectionnée     |
| `p`          | Lecture / Pause                  |
| `n`          | Piste suivante                   |
| `b`          | Piste précédente                 |
| `←` / `→`    | Reculer / Avancer de 5 secondes  |
| `↑` / `↓`    | Naviguer dans la playlist        |
| `e`          | Mode saisie (recherche)          |
| `Entrée`     | Valider la recherche             |
| `Échap`      | Quitter le mode saisie           |
| `c`          | Effacer la recherche             |
| `q`          | Quitter                          |

---

## Stack technique

| Crate      | Rôle                                  |
|------------|---------------------------------------|
| `ratatui`  | Interface TUI (widgets, layout)       |
| `rodio`    | Moteur de lecture audio               |
| `ringbuf`  | Ring buffers lock-free pour l'audio   |
| `rustfft`  | Transformée de Fourier (FFT)          |
| `apodize`  | Fenêtrage de Hanning pour la FFT      |
| `crossterm`| Gestion du terminal cross-platform    |

---

## Commandes de développement

```bash
cargo build           # Compilation debug
cargo build --release # Compilation optimisée
cargo run             # Lancer en mode debug
cargo test            # Tests unitaires
cargo fmt             # Formatter le code
cargo clippy          # Linter
```

---

## Ce que j'ai appris

Ce projet m'a permis d'explorer en profondeur la gestion de la concurrence en Rust : coordination de plusieurs threads avec `Arc` pour l'arrêt gracieux, isolation des données audio via des ring buffers pour éviter la contention sur le mutex, et implémentation d'une analyse spectrale temps réel sans impacter la fluidité de l'UI.

---

## Formats supportés

MP3, FLAC, WAV (via `rodio` + `symphonia`)

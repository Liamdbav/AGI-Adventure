# AGI Adventure

RPG monster-tamer futuriste-fantasy en Rust. Le joueur doit sauver l'AGI mondiale des **ShinyHunterz**, un groupe de hackers qui tente de réécrire l'histoire de l'humanité via une injection de prompt à l'échelle planétaire.

## Prérequis

- **Rust stable ≥ 1.85** — [rustup.rs](https://rustup.rs)
- **macOS** : Xcode Command Line Tools (`xcode-select --install`)
- **Linux** : `libasound2-dev` et `libx11-dev` (ex. `sudo apt install libasound2-dev libx11-dev`)
- **Windows** : aucune dépendance supplémentaire

## Lancer le jeu

```bash
cargo run
```

## Build release

```bash
cargo build --release
# binaire dans target/release/agi-adventure
```

## Contrôles

| Action | Touches |
|---|---|
| Déplacement | Flèches / ZQSD (AZERTY) / WASD |
| Interaction / dialogue | Espace |
| Inventaire | I |
| Naviguer inventaire | ↑↓ ou W/S |
| Utiliser objet | Espace |
| Fermer menu | Échap |
| Sauvegarder | F5 |
| Quitter | Échap |

## Stack technique

- [Rust](https://www.rust-lang.org/) 2024 edition
- [Macroquad](https://macroquad.rs/) 0.4 — rendu, input, audio
- [tiled](https://docs.rs/tiled) 0.12 — maps TMX/TSX
- [serde_json](https://docs.rs/serde_json) — données de jeu et sauvegarde
- [rand](https://docs.rs/rand) 0.8 — RNG

Résolution logique : 160×144 px (Game Boy Color). Fenêtre physique : 640×576 px (×4).

## License

MIT — voir [LICENSE](LICENSE)

# AGI Adventure — CLAUDE.md

## Lore & univers
AGI Adventure est un RPG monster-tamer futuriste-fantasy dans lequel
le joueur doit sauver l'AGI mondiale (intelligence artificielle
générale) d'un groupe de hackers nommé les **ShinyHunterz**, qui
tentent de corrompre les systèmes d'information planétaires pour
réécrire l'histoire de l'humanité via une injection de prompt à
l'échelle mondiale.

L'univers mêle culture internet, hacking, indie hacker, sysadmin,
et références à la culture tech populaire (Linus Torvalds, Edward
Snowden, Satoshi Nakamoto, Elon Musk, HuggingFace, Amazon, LeBonCoin).
Le ton est à la limite du troll mais immersif par la densité de ses
références. Le joueur nomme son personnage au démarrage.

## PNJ implémentés
- **Groki** : PNJ narratif. Référence à Grok (IA de xAI). Omniscient,
  sarcastique, parle par références tech et blagues de mauvais goût.
  Rôle : guide/informateur, premier contact du joueur à Nebular City.
- **Script Kiddies** : premier dresseur antagoniste. Utilise des outils
  sans les comprendre, se la raconte. Équipe : Bob niveau 5.
  Dialogue de défaite affiché dans l'écran de victoire du combat.

## Créatures implémentées
- **Bob** : type Prompt. Moves : Surcharge, Guardrail. catch_rate: 255.
- **Alice** : type Brute. Moves : Injection, Guardrail. catch_rate: 180.
- **John** : type Exploit. Moves : Surcharge, Trend Effect. catch_rate: 200.

Toutes les créatures sont dans `assets/data/creatures.json`.
Formule de stats : `(base * lv) / 50 + bonus` (HP : +lv+10, autres : +5).

## Attaques implémentées
- **Surcharge** : 25 dégâts, type Prompt, physical.
- **Guardrail** : 0 dégâts, status. Réduit les dégâts reçus de 30%
  pendant 3 tours (effect: defense_boost). Décrémenté chaque fin de tour.
- **Injection** : 60 dégâts, type Exploit, special. PP: 5.
- **Trend Effect** : 0 dégâts, status. Réduit la vitesse ennemie de 30%
  pendant 3 tours (effect: speed_down). Décrémenté chaque fin de tour.

Toutes les attaques sont dans `assets/data/attacks.json`.

## Objets implémentés
- **Patch** (`patch`) : soin de 20 HP. Utilisable en combat et en exploration.
- **CTF** (`capture_flag`) : capture une créature sauvage. Taux = catch_rate/255.
  Interdit en combat dresseur. Inventaire de départ : 3 Patch, 5 CTF.

Tous les objets sont dans `assets/data/items.json`.

## Maps implémentées
- **Nebular City** (`town`) : ville de départ. Fichier : `assets/maps/town.tmx`.
- **Wiki Road** (`route1`) : herbe haute = zones de rencontre sauvage.
  Fichier : `assets/maps/route1.tmx`.

Warp town→route1 : tile (5,0). Warp route1→town : tile (5,8).
Propriété Tiled `encounter_zone=true` sur les tuiles d'herbe haute.

## Stack technique
- Rust stable + Cargo (edition 2024)
- Macroquad 0.4 pour le rendu, input, audio
- crate `tiled` 0.12 pour les maps TMX/TSX (chargement synchrone)
- serde + serde_json pour les données de jeu et la sauvegarde
- Résolution logique : 160×144 px (Game Boy Color)
- Fenêtre physique : 640×576 px (scale ×4) — `WINDOW_SCALE` dans `main.rs`

## Structure du projet
```
agi-adventure/
├── src/
│   ├── main.rs       — entrypoint, boucle de jeu, GameState, warps
│   ├── world.rs      — chargement et rendu des tilemaps TMX
│   ├── player.rs     — état, déplacement tile-by-tile, détection herbe
│   ├── creature.rs   — CreatureInstance, from_data(), stats
│   ├── combat.rs     — CombatState, TurnPhase, CombatOutcome, rendu
│   ├── npc.rs        — NpcInstance, DialogueResult, boîte de dialogue
│   ├── inventory.rs  — Inventory, render_menu(), from_items()
│   ├── save.rs       — SaveData (serde), save(), load(), from_game_state()
│   └── data.rs       — GameData, chargement des JSON au démarrage
├── assets/
│   ├── maps/         — town.tmx, route1.tmx
│   ├── tilesets/     — fichiers .tsx + images tileset .png
│   ├── sprites/      — player.png, bob.png, alice.png, john.png, npc_generic.png
│   └── data/         — creatures.json, attacks.json, items.json, npcs.json
├── CLAUDE.md
├── Cargo.toml
└── save.json         — généré au runtime (ignoré par git)
```

## GameState (machine d'états)
```
NameEntry → Explore ⟷ InventoryOverlay
                 ↓ (PNJ)
              Dialogue → CombatTrainer
                 ↓ (herbe)
              CombatWild
```
- `NameEntry` : saisie du nom. Sauté si `save.json` existe.
- `Explore` : déplacement, interactions, warps.
- `InventoryOverlay` : overlay inventaire sur la scène d'exploration.
- `Dialogue` : boîte de dialogue PNJ. `TriggerBattle` → `CombatTrainer`.
- `CombatWild` / `CombatTrainer` : combat au tour par tour.
- `GameOver` : écran de fin. Espace → warp town (5,4), HP full → Explore.

## Système de combat
- **TurnPhase** : `PlayerChoose`, `ItemMenu { selected }`, `ShowResult`,
  `Victory`, `Defeat`.
- **CombatOutcome** : `Ongoing`, `Exit { final_player_hp, victory }`,
  `Captured { final_player_hp, captured_name }`.
- Slot "Bag" dans la grille de moves (index `moves.len()` si < 4).
- Initiative : le plus rapide attaque en premier (speed_down_turns réduit
  la vitesse de 30%).
- Formule dégâts : `(atk * power) / (def_eff * 2) + 2`, min 1.
  `def_eff = def * 130/100` si Guardrail actif, sinon `def` brut.
  Guardrail divise les dégâts reçus par ~1.43 (×0.7) pendant 3 tours.
- `trainer_defeat_quote` : affiché dans le log de victoire contre un dresseur.

## Système de sauvegarde
- Fichier : `save.json` (JSON lisible, généré à côté du binaire).
- Contenu : `player_name`, `player_tile_x/y`, `current_map`,
  `player_creature` (CreatureInstance sérialisée), `inventory`
  (HashMap), `defeated_trainers` (Vec de IDs).
- **F5** en Explore → sauvegarde + flash "Sauvegarde effectuée." (1,5 s).
- Au démarrage : si `save.json` présent → restaure tout, saute NameEntry.
- Après un warp : `apply_defeated_trainers()` remet les PNJ dans le bon état.

## Contrôles
- **Déplacement** : flèches ou ZQSD (AZERTY) / WASD
- **Interaction / avancer dialogue** : Espace
- **Inventaire (exploration)** : I
- **Naviguer inventaire** : ↑↓ ou W/S, Espace pour utiliser, Échap pour fermer
- **Sauvegarder** : F5
- **Quitter** : Échap (depuis n'importe quel état)

## Règles de développement
- Chaque module a une responsabilité unique.
- Les données de jeu sont TOUJOURS lues depuis `assets/data/` — jamais
  hardcodées (sauf la créature de départ du joueur, temporairement).
- La résolution logique 160×144 est sacrée. Tous les draw calls de scène
  passent par le `render_target` 160×144, puis blittés ×4 vers la fenêtre
  physique (`flip_y: true`). Ne JAMAIS dessiner en coordonnées fenêtre
  sauf pour les overlays UI (inventaire, dialogue, combat).
- Pas de `unwrap()` — utiliser `expect()` avec message explicite.
- Chaque struct sérialisable dérive `Serialize` et `Deserialize`.
- `CombatOutcome` est `Clone` (pas `Copy`) car le variant `Captured`
  contient un `String`.

## Commandes utiles
```
cargo run             — lancer en développement
cargo build --release — binaire optimisé macOS
cargo check           — vérifier sans compiler
```

## État du MVP
✅ Nebular City + Wiki Road avec warp bidirectionnel
✅ 3 créatures (Bob/Alice/John), 4 attaques
✅ Rencontres sauvages dans l'herbe
✅ 2 PNJ (Groki + Script Kiddies), dialogue et combat dresseur
✅ Inventaire avec Patch et CTF (exploration + combat)
✅ Sauvegarde/chargement JSON (F5)

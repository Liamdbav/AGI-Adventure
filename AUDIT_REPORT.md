# Rapport d'audit — AGI Adventure
**Date :** 2026-05-18
**Stack :** Rust 2024 edition, Macroquad 0.4, tiled 0.12, serde / serde_json
**Auditeur :** Claude Code — mode audit

---

## Résumé exécutif

AGI Adventure est un MVP de RPG monster-tamer en Rust/Macroquad fonctionnellement complet pour son périmètre (deux maps, trois créatures, combat tour-par-tour, sauvegarde JSON). Le projet compile sans erreur, est structuré en modules cohérents et le gameplay tourne. Néanmoins, en l'état il **n'est pas livrable** : aucun README ni licence, aucun test, vingt `expect()` qui transforment chaque erreur d'I/O ou de parsing en crash, et un dépôt git encore vide (aucun commit). Plusieurs incohérences fonctionnelles existent entre la documentation et le code (formule de dégâts, état `GameOver` jamais atteint). Le risque pour la livraison est **modéré à élevé** : le binaire fonctionne en happy-path, mais le moindre asset manquant ou JSON malformé produit une panique. Verdict : retravailler la robustesse des chargements, ajouter une doc minimale et purger les fichiers parasites avant remise.

## Score de livraison : 5/10

| Dimension | Score | Statut |
|-----------|-------|--------|
| Sécurité | 7/10 | 🟡 |
| Stabilité | 4/10 | 🔴 |
| Performance | 6/10 | 🟡 |
| Qualité de code | 5/10 | 🟠 |
| Tests & Observabilité | 1/10 | 🔴 |
| Documentation | 3/10 | 🟠 |

---

## Problèmes identifiés

### 🔴 DANGERS (5 problèmes)

#### [DANGER-001] Chargements d'assets avec `expect()` — crash garanti sur fichier manquant
**Fichier :** `src/main.rs:86-88`, `src/world.rs:22-40`, `src/player.rs:44-46`, `src/npc.rs:24-27`, `src/data.rs:73-89`
**Observation :** 20 appels à `expect()` au démarrage et pendant les warps. Toute absence ou corruption d'un asset (`images/logo-agi.png`, `assets/maps/*.tmx`, `assets/sprites/*.png`, `assets/data/*.json`) panique le binaire. CLAUDE.md interdit `unwrap()` mais autorise `expect()` — or `expect()` panique exactement comme `unwrap()`.
**Risque :** Crash sans message utilisateur dès qu'un fichier est manquant, déplacé ou corrompu côté client. Aucun écran d'erreur, aucun log structuré, le jeu se ferme brutalement.
**Solution :** Centraliser le chargement d'assets dans une fonction qui retourne `Result<_, String>`, afficher un écran d'erreur en cas d'échec, et tolérer les fichiers absents (par exemple `dialogue_defeated` est déjà `Option<Vec<String>>`, étendre ce pattern). À défaut, au minimum logger via `eprintln!` un chemin absolu avant `expect()`.

#### [DANGER-002] `save.json` versionné et contient le nom réel du joueur
**Fichier :** `save.json` (racine du dépôt), `.gitignore`
**Observation :** `save.json` est présent à la racine, contient `"player_name": "LiamDBAV"`, et n'est PAS dans `.gitignore` (qui ne contient que `/target`). CLAUDE.md indique pourtant « `save.json` — généré au runtime (ignoré par git) ».
**Risque :** Le fichier sera commité par mégarde et expédié au client avec les données d'une session locale. Brouille la première exécution chez le client (le jeu sautera la création de personnage). Fuite mineure d'un identifiant.
**Solution :** Ajouter `save.json` au `.gitignore` et supprimer le fichier de la racine avant la livraison :
```
echo -e "save.json\n.DS_Store\n.idea/\n.vscode/\n*.log" >> .gitignore
rm save.json
```

#### [DANGER-003] Dépôt git sans aucun commit
**Fichier :** racine du dépôt
**Observation :** `git log` retourne « no commits yet ». Tous les fichiers du projet sont untracked.
**Risque :** Aucun historique, aucune trace de qui a fait quoi, impossibilité pour le client de récupérer une version stable, aucun fallback en cas de régression. Si une suppression accidentelle survient avant livraison, la perte est totale.
**Solution :** Initialiser proprement l'historique, créer un commit initial étiqueté `v0.1.0-mvp`, pousser sur un remote (GitHub privé, GitLab) avant remise au client.

#### [DANGER-004] Sauvegarde non atomique — corruption possible
**Fichier :** `src/save.rs:19-22`
**Observation :** `save()` fait `std::fs::write(path, json)`. Si le processus est tué (Cmd+Q forcé, crash, coupure secteur) pendant l'écriture, `save.json` est tronqué et la prochaine lecture échouera silencieusement (`ok()?`) — toute la progression est perdue sans avertissement.
**Risque :** Perte de progression silencieuse. C'est typiquement le bug qu'un client appelle « le jeu efface mes parties ».
**Solution :** Écrire dans `save.json.tmp` puis `std::fs::rename` :
```rust
let tmp = format!("{}.tmp", path);
std::fs::write(&tmp, json).expect("écriture save.json.tmp échouée");
std::fs::rename(&tmp, path).expect("renommage save.json échoué");
```
Et idéalement notifier l'utilisateur en cas d'échec de chargement plutôt que de retourner `None` silencieusement.

#### [DANGER-005] Formule de dégâts incohérente entre code et doc — comportement non spécifié
**Fichier :** `src/combat.rs:343`, `CLAUDE.md` section « Système de combat »
**Observation :** CLAUDE.md spécifie `(attaque * puissance) / (défense * 10)`, mais le code applique `(atk_power_stat * atk.power) / (eff_defense * 2) + 2`. Avec Bob lv5 (atk=9, def=9) attaquant Bob lv5 avec Surcharge (power=25) : doc → 9*25/(9*10) = 2 dégâts ; code → 9*25/(9*2)+2 = 14 dégâts. Le ratio est différent d'un facteur 7.
**Risque :** Spec contractuelle floue : le client ne sait pas quelle est la bonne formule, et un futur correctif peut casser l'équilibrage. C'est aussi un risque réputationnel (« la doc ment »).
**Solution :** Aligner code et doc. Décider laquelle des deux formules est canonique, mettre l'autre à jour, et documenter une matrice de tests d'équilibrage (Bob lv5 vs Bob lv5 → X dégâts attendus).

---

### 🟠 MANQUEMENTS (8 problèmes)

#### [MANQUE-001] Aucun README
**Fichier :** racine
**Observation :** Aucun `README.md` à la racine. Seul `CLAUDE.md` documente le projet, mais il s'adresse à l'agent d'IA, pas au client ni à un nouveau développeur.
**Impact :** Le client ne sait pas comment lancer le jeu, sur quelles plateformes, ni à quoi s'attendre. Aucune capture d'écran, aucun mini-pitch.
**Solution :** Créer un `README.md` minimal avec : pitch (5 lignes), captures, prérequis (`rustup` stable), commandes (`cargo run`, `cargo build --release`), contrôles, licence, support.

#### [MANQUE-002] Aucune licence
**Fichier :** racine
**Observation :** Pas de `LICENSE` ni de champ `license` dans `Cargo.toml`. Statut juridique du code = propriétaire implicite, mais flou (assets, dépendances OSS, références culturelles…).
**Impact :** Le client ne sait pas ce qu'il a le droit d'en faire. Bloquant si livraison commerciale.
**Solution :** Décider d'une licence (MIT, GPL, propriétaire avec mention de copyright). Ajouter `LICENSE` et le champ `license = "..."` dans `Cargo.toml`. Vérifier la compatibilité avec macroquad (MIT/Apache-2.0) et tiled (MIT).

#### [MANQUE-003] Aucun test
**Fichier :** projet entier
**Observation :** `grep "#[test]"` ne renvoie rien. Aucune crate de test, aucun module `#[cfg(test)]`. Les invariants critiques (formule de dégâts, taux de capture, transitions de `GameState`, restauration de sauvegarde) ne sont vérifiés que manuellement.
**Impact :** Aucune garantie qu'un refactoring ne casse pas le gameplay. Aucune régression détectable en CI.
**Solution :** Ajouter au minimum des tests unitaires pour :
- `CreatureInstance::from_data` (stats calculées correctement à chaque niveau)
- `Inventory::use_item` (épuisement, ordre)
- `SaveData::save` puis `SaveData::load` round-trip
- Calcul des dégâts (combat.rs apply_attack — extraire la formule pure)

#### [MANQUE-004] État `GameOver` jamais atteint — pas de fail state
**Fichier :** `src/main.rs:477`, `src/combat.rs:248`
**Observation :** L'enum `GameState::GameOver` existe (`#[allow(dead_code)]` masque les warnings), un écran "GAME OVER" est dessiné, mais le code de défaite ramène toujours le joueur à `Explore` avec `final_player_hp = max_hp / 2`. Le jeu est techniquement infini.
**Impact :** Pas de tension narrative, pas de risque réel pour le joueur, l'écran GAME OVER est mort. Le client paie pour une feature inexistante.
**Solution :** Soit retirer l'état et le rendu (et documenter le choix « pas de game over dans le MVP »), soit câbler le scénario : après défaite contre un dresseur, retour au point de spawn de la map de départ avec HP=max et un message.

#### [MANQUE-005] `.gitignore` famélique
**Fichier :** `.gitignore`
**Observation :** Contenu : `/target` seulement. Manquent `save.json`, `.DS_Store` (présent partout : racine, `assets/`), `images/logo-agi.jpg` (doublon JPG inutilisé), `*.log`, `.idea/`, `.vscode/`.
**Impact :** Pollution future du dépôt, fuite de métadonnées macOS (.DS_Store), doublons d'assets.
**Solution :** Voir DANGER-002. Compléter aussi avec `**/.DS_Store` et supprimer les `.DS_Store` actuellement présents (`find . -name .DS_Store -delete`).

#### [MANQUE-006] Variables d'environnement et procédure d'installation non documentées
**Fichier :** projet entier
**Observation :** Pas de `.env.example`, aucune doc des prérequis (rustup, version stable minimale, dépendances système macOS/Linux/Windows pour macroquad qui requiert OpenGL ou Metal). Pas de doc de build release pour les trois plateformes.
**Impact :** Le client ne peut pas reproduire un build. Sur Windows ou Linux il rencontrera potentiellement des dépendances système non installées.
**Solution :** Documenter dans le README :
- Version minimale de Rust (stable, edition 2024 → 1.85+)
- Sur Linux : `libasound2-dev`, `libudev-dev`
- Sur macOS : Xcode CLT
- Commandes de build cross-platform : `cargo build --release --target x86_64-pc-windows-gnu` etc.

#### [MANQUE-007] Aucun logging structuré — débogage opaque côté client
**Fichier :** `src/main.rs:78,459`
**Observation :** Deux `println!` éparpillés (chargement de GameData, capture). Aucune crate de log (`log`, `tracing`), aucun niveau, aucun format reproductible.
**Impact :** Quand le client signale un bug, impossible de demander un fichier de log. Le débogage repose sur la reproductibilité chez le développeur.
**Solution :** Ajouter `log = "0.4"` + `env_logger = "0.11"`, remplacer les `println!` par `log::info!`/`warn!`/`error!`, ajouter des `error!` aux endroits actuellement gardés par `expect()`.

#### [MANQUE-008] PP affiché statique — feature incomplète
**Fichier :** `src/combat.rs:592`
**Observation :** `draw_text(&format!("PP  {}/{}", pp, pp), ...)` — le « PP actuel » est toujours égal au max. Aucune logique de décrémentation de PP n'est implémentée dans `apply_attack`, alors que le champ `pp` est lu depuis les données.
**Impact :** Feature visible mais non fonctionnelle. Le joueur peut spammer une attaque à 5 PP indéfiniment. Incohérence ressentie comme un bug par le client.
**Solution :** Soit retirer l'affichage des PP du HUD, soit câbler la décrémentation : ajouter `current_pp: Vec<u32>` à `CreatureInstance`, décrémenter dans `apply_attack` quand `player_attacks`, bloquer le choix si 0.

---

### 🟡 OPTIMISATIONS (7 points)

#### [OPTI-001] Rechargement complet de map et tileset à chaque warp
**Fichier :** `src/main.rs:149,209`, `src/world.rs:19-101`
**Observation :** Chaque warp re-parse le TMX, relit l'image du tileset depuis le disque et la ré-upload sur le GPU (`Texture2D::from_file_with_format`). Pour 2 maps c'est négligeable, mais l'architecture ne passe pas l'échelle.
**Gain estimé :** Micro-freeze de 30-150 ms supprimé à chaque warp, économie de RAM/VRAM si beaucoup de maps.
**Solution :** Créer un `MapCache: HashMap<String, GameMap>` initialisé au démarrage, lookup au warp. Tilesets partagés via un `Rc<Texture2D>` si plusieurs maps utilisent le même.

#### [OPTI-002] Sprite NPC rechargé du disque pour chaque NPC
**Fichier :** `src/npc.rs:23-28`
**Observation :** `NpcInstance::new` fait `std::fs::read("assets/sprites/npc_generic.png")` + `Texture2D::from_file_with_format` pour chaque instance. Avec 2 NPCs c'est marginal, mais structurellement faux.
**Gain estimé :** Mémoire et I/O divisés par N. À grande échelle (10+ NPCs par map) c'est sensible.
**Solution :** Charger les textures NPC une seule fois dans une `TextureBank` passée à `NpcInstance::new`, ou utiliser un sprite_id et un cache global de textures.

#### [OPTI-003] Rendu de tilemap O(W×H×couches) sans culling
**Fichier :** `src/world.rs:103-153`
**Observation :** Chaque frame, double boucle sur toute la map pour chaque layer + canopy, même pour les tuiles vides (`continue` après lookup). Pour 160×144 logique et tuiles 16px → ~10×9 = 90 tuiles visibles, ce n'est pas grave ici, mais aucun culling caméra n'existe.
**Gain estimé :** Insignifiant à cette résolution. Préventif uniquement.
**Solution :** Documenter explicitement « pas de culling, OK car la map tient dans le viewport ». Sinon ajouter un range basé sur l'offset caméra.

#### [OPTI-004] Logo PNG dupliqué et lourd (781 KB × 2)
**Fichier :** `assets/sprites/logo-agi.png`, `images/logo-agi.png`, `images/logo-agi.jpg`
**Observation :** Le logo est présent en double (782 KB chacun) plus une version JPG (169 KB) dans `images/`. `main.rs` lit `images/logo-agi.png` ; `assets/sprites/logo-agi.png` est inutilisé.
**Gain estimé :** -950 KB sur le binaire de livraison.
**Solution :** Supprimer `assets/sprites/logo-agi.png` et `images/logo-agi.jpg`. Optimiser le PNG restant avec `oxipng` ou `pngquant` (typiquement -60% à -80% sur un logo de cette taille).

#### [OPTI-005] RNG bricolé sur `get_time()` — biais et déterminisme partiel
**Fichier :** `src/player.rs:82`, `src/combat.rs:168,183,200,271`
**Observation :** Tous les aléas du jeu utilisent `(get_time() * constante) as u64`. Si deux appels surviennent dans la même frame, ils sont fortement corrélés. Le modulo introduit en plus un biais modulo classique.
**Gain estimé :** Qualité de gameplay : capture/rencontres/IA ennemie cessent d'avoir des patterns visibles.
**Solution :** Ajouter `rand = "0.8"` + `rand_pcg`, instancier un `Pcg32` une fois au démarrage, passer une `&mut Rng` aux fonctions qui en ont besoin.

#### [OPTI-006] `main.rs` à 1065 lignes — module monolithique
**Fichier :** `src/main.rs`
**Observation :** 1065 lignes dont la moitié est du rendu d'UI (title screen, name entry, pause menu, character sheet, map, settings). La logique de transition GameState est noyée. CLAUDE.md précise « chaque module a une responsabilité unique » — `main.rs` viole ce principe.
**Gain estimé :** Maintenance : isoler un bug d'UI sera plus rapide. Compilation incrémentale plus fine.
**Solution :** Extraire un module `src/ui.rs` (toutes les fonctions `draw_*`) et un module `src/state_machine.rs` ou similaire pour la logique de transitions. Garder `main.rs` à ~300 lignes.

#### [OPTI-007] 30 warnings clippy non traités
**Fichier :** divers (24× collapsible_if, 1× ptr_arg, 1× saturating_sub implicite, 1× is_multiple_of, 1× explicit closure for copying)
**Observation :** `cargo clippy --no-deps` rapporte 30 warnings. La plupart sont mineurs (collapsible if), mais `ptr_arg` (`&mut Vec` au lieu de `&mut [_]`) et `expect()` avec `format!` à l'intérieur (`data.rs:99,107`) sont des hygiène-points.
**Gain estimé :** Lisibilité, un peu de perf (format! évalué à chaque appel, pas seulement en cas de panique).
**Solution :** `cargo clippy --fix --bin agi-adventure --allow-dirty` applique 29 des 30 corrections automatiquement. Pour les `expect(&format!(...))` → `unwrap_or_else(|| panic!("..."))`.

---

## Plan d'action prioritaire

1. [DANGER-002] + [MANQUE-005] — Nettoyer `.gitignore`, supprimer `save.json` et les `.DS_Store` — ⏱️ < 30min
2. [DANGER-003] — Initialiser l'historique git, faire un commit initial, pousser sur un remote — ⏱️ < 30min
3. [MANQUE-001] + [MANQUE-002] + [MANQUE-006] — Rédiger README + LICENSE + procédure d'install multi-plateforme — 🕐 < 2h
4. [DANGER-001] — Centraliser le chargement d'assets et afficher un écran d'erreur au lieu de paniquer — 🕐 < 2h
5. [DANGER-005] + [MANQUE-004] + [MANQUE-008] — Aligner doc et code (formule de dégâts), décider du sort de GameOver et des PP — 🕐 < 2h
6. [DANGER-004] — Rendre l'écriture de `save.json` atomique (write tmp + rename) — ⏱️ < 30min
7. [MANQUE-003] — Ajouter une couverture de tests minimale sur les invariants critiques — 📅 > 2h
8. [OPTI-007] — `cargo clippy --fix` pour purger les 30 warnings — ⏱️ < 30min

---

## Points positifs

- **Architecture modulaire claire** : `combat`, `world`, `player`, `npc`, `inventory`, `save`, `data` ont chacun une responsabilité bien définie (sauf `main.rs`).
- **Données externalisées** : créatures, attaques, items, NPCs sont en JSON dans `assets/data/`, comme spécifié dans CLAUDE.md. Bonne séparation contenu/code.
- **Compile proprement** : `cargo check` passe, pas d'erreur, pas d'`unsafe`, pas de dépendances exotiques. La stack est sobre (4 crates).
- **Résolution logique 160×144 respectée** via `render_target` et blit ×4 — l'esthétique Game Boy Color est techniquement bien implémentée.
- **Sérialisation propre** : `serde` + `Serialize/Deserialize` dérivés systématiquement, round-trip JSON lisible et inspectable.
- **Gestion des collisions et zones de rencontre** via propriétés Tiled (`collidable`, `encounter_zone`) — clean, extensible aux futures maps sans recompilation.
- **Système de menu pause complet** (Inventaire / Personnage / Carte / Sauvegarder / Paramètres) avec confirmation pour les actions destructrices.
- **Animation des transitions** : déplacement tile-by-tile interpolé, bandeau de map, flash de sauvegarde — petites attentions qui rendent le jeu agréable.
- **Pas de panique runtime observée** sur le happy-path actuel : le binaire démarre, joue, sauvegarde, recharge sans broncher tant que les assets sont là.

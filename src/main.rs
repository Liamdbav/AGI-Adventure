mod combat;
mod creature;
mod data;
mod inventory;
mod npc;
mod player;
mod save;
mod world;

use macroquad::prelude::*;
use ::rand::Rng;

use crate::combat::{CombatOutcome, CombatState};
use crate::inventory::Inventory;
use crate::creature::CreatureInstance;
use crate::data::{AttackData, GameData};
use crate::npc::{DialogueResult, NpcInstance, load_npc_texture};
use crate::player::{Direction, Player};
use crate::save::SaveData;
use crate::world::GameMap;

const LOGICAL_W: f32 = 160.0;
const LOGICAL_H: f32 = 144.0;
const WINDOW_SCALE: i32 = 4;
const PHYS_W: f32 = LOGICAL_W * WINDOW_SCALE as f32;
const PHYS_H: f32 = LOGICAL_H * WINDOW_SCALE as f32;

fn window_conf() -> Conf {
    Conf {
        window_title: "AGI Adventure".to_owned(),
        window_width: PHYS_W as i32,
        window_height: PHYS_H as i32,
        window_resizable: false,
        ..Default::default()
    }
}

pub struct PlayerProfile {
    pub name: String,
}

enum GameState {
    TitleScreen,
    NameEntry { buffer: String },
    Explore,
    InventoryOverlay { selected: usize },
    Dialogue { npc_index: usize },
    CombatWild { combat: CombatState },
    CombatTrainer { trainer_id: String, combat: CombatState },
    GameOver,
    PauseMenu { selected: usize },
    PauseInventory { selected: usize },
    PauseCharacter,
    PauseMap,
    PauseSettings { selected: usize, confirm_delete: bool },
}

struct Warp {
    from_map: &'static str,
    trigger_x: i32,
    trigger_y: i32,
    to_map: &'static str,
    to_map_path: &'static str,
    spawn_x: i32,
    spawn_y: i32,
    display_name: &'static str,
}

/// Affiche un écran d'erreur fatal et quitte si Échap est pressé.
async fn fatal_error(msg: &str) {
    loop {
        clear_background(BLACK);
        draw_text("ERREUR CRITIQUE", PHYS_W * 0.5 - 120.0, PHYS_H * 0.5 - 40.0, 28.0, RED);
        let wrapped = simple_wrap(msg, 60);
        for (i, line) in wrapped.iter().enumerate() {
            draw_text(line, 16.0, PHYS_H * 0.5 + i as f32 * 24.0, 18.0, WHITE);
        }
        draw_text("[ Échap ] Quitter", 16.0, PHYS_H - 30.0, 16.0, GRAY);
        if is_key_pressed(KeyCode::Escape) {
            std::process::exit(1);
        }
        next_frame().await;
    }
}

fn simple_wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn create_npcs(game_data: &GameData, map_id: &str, npc_texture: &Texture2D) -> Vec<NpcInstance> {
    game_data
        .get_npcs_for_map(map_id)
        .into_iter()
        .cloned()
        .map(|d| NpcInstance::new(d, npc_texture.clone()))
        .collect()
}

#[macroquad::main(window_conf)]
async fn main() {
    let target = render_target(LOGICAL_W as u32, LOGICAL_H as u32);
    target.texture.set_filter(FilterMode::Nearest);

    let mut scene_cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, LOGICAL_W, LOGICAL_H));
    scene_cam.render_target = Some(target.clone());

    let game_data = match GameData::load() {
        Ok(d) => {
            eprintln!(
                "GameData chargé : {} créatures, {} attaques, {} objets, {} NPCs",
                d.creatures.len(), d.attacks.len(), d.items.len(), d.npcs.len()
            );
            d
        }
        Err(e) => { fatal_error(&e).await; return; }
    };

    let logo_texture = match std::fs::read("images/logo-agi.png") {
        Ok(bytes) => {
            let t = Texture2D::from_file_with_format(&bytes, Some(ImageFormat::Png));
            t.set_filter(FilterMode::Linear);
            t
        }
        Err(e) => {
            eprintln!("logo introuvable : {e}");
            let t = Texture2D::from_image(&Image::gen_image_color(320, 288, BLACK));
            t.set_filter(FilterMode::Linear);
            t
        }
    };

    let npc_texture = load_npc_texture();

    let mut map = match GameMap::load("assets/maps/town.tmx") {
        Ok(m) => m,
        Err(e) => { fatal_error(&e).await; return; }
    };
    let mut current_map = "town".to_string();

    let warps: &[Warp] = &[
        Warp {
            from_map: "town",    trigger_x: 5, trigger_y: 0,
            to_map: "route1",    to_map_path: "assets/maps/route1.tmx",
            spawn_x: 5,          spawn_y: 7,
            display_name: "Wiki Road",
        },
        Warp {
            from_map: "route1",  trigger_x: 5, trigger_y: 8,
            to_map: "town",      to_map_path: "assets/maps/town.tmx",
            spawn_x: 5,          spawn_y: 1,
            display_name: "Nebular City",
        },
    ];

    let mut npcs: Vec<NpcInstance> = create_npcs(&game_data, "town", &npc_texture);

    let mut map_banner: Option<(String, f32)> = None;
    let mut save_data: Option<SaveData> = SaveData::load("save.json");
    let mut state = GameState::TitleScreen;
    let mut profile: Option<PlayerProfile> = None;
    let mut player: Option<Player> = None;
    let mut player_creature: Option<CreatureInstance> = None;
    let mut inventory = Inventory::new();
    let mut defeated_trainers: Vec<String> = Vec::new();
    let mut save_flash: f32 = 0.0;

    'game: loop {
        let dt = get_frame_time();
        if save_flash > 0.0 { save_flash -= dt; }

        let npc_tiles: Vec<(i32, i32)> = npcs
            .iter()
            .map(|n| (n.data.tile_x, n.data.tile_y))
            .collect();

        let mut combat_exit: Option<(u32, bool)> = None;
        let mut combat_trainer_id: Option<String> = None;
        let mut pause_save_requested = false;
        let mut pause_delete_save_requested = false;
        match &mut state {
            GameState::TitleScreen => {
                if is_key_pressed(KeyCode::Escape) {
                    break 'game;
                }
                if is_key_pressed(KeyCode::Enter) {
                    if let Some(sv) = save_data.take() {
                        current_map = sv.current_map.clone();
                        map = match GameMap::load(map_path_for_id(&sv.current_map)) {
                            Ok(m) => m,
                            Err(e) => { fatal_error(&e).await; return; }
                        };
                        npcs = create_npcs(&game_data, &sv.current_map, &npc_texture);
                        defeated_trainers = sv.defeated_trainers;
                        apply_defeated_trainers(&mut npcs, &defeated_trainers);
                        profile = Some(PlayerProfile { name: sv.player_name });
                        player = Some(Player::new(sv.player_tile_x, sv.player_tile_y));
                        player_creature = Some(sv.player_creature);
                        inventory = Inventory::from_items(sv.inventory);
                        state = GameState::Explore;
                    } else {
                        state = GameState::NameEntry { buffer: String::new() };
                    }
                }
            }

            GameState::NameEntry { buffer } => {
                while let Some(c) = get_char_pressed() {
                    if c == '\u{8}' {
                        buffer.pop();
                    } else if c.is_control() {
                        // ignorer
                    } else if buffer.len() < 12 {
                        buffer.push(c);
                    }
                }
                if is_key_pressed(KeyCode::Enter) && !buffer.is_empty() {
                    let name = buffer.clone();
                    profile = Some(PlayerProfile { name });
                    player = Some(Player::new(5, 4));
                    player_creature = Some(CreatureInstance::from_data(
                        game_data.get_creature("bob"), 5,
                    ));
                    state = GameState::Explore;
                }
            }

            GameState::Explore => {
                let mut triggered_warp: Option<usize> = None;
                if let Some(p) = player.as_mut() {
                    let stepped = p.update(&map, &npc_tiles, dt);
                    if stepped {
                        for (i, w) in warps.iter().enumerate() {
                            if w.from_map == current_map
                                && p.tile_x == w.trigger_x
                                && p.tile_y == w.trigger_y
                            {
                                triggered_warp = Some(i);
                                break;
                            }
                        }
                    }
                }

                if let Some(i) = triggered_warp {
                    let w = &warps[i];
                    map = match GameMap::load(w.to_map_path) {
                        Ok(m) => m,
                        Err(e) => { fatal_error(&e).await; return; }
                    };
                    current_map = w.to_map.to_string();
                    npcs = create_npcs(&game_data, w.to_map, &npc_texture);
                    apply_defeated_trainers(&mut npcs, &defeated_trainers);
                    map_banner = Some((w.display_name.to_string(), 2.0));
                    if let Some(p) = player.as_mut() {
                        p.teleport(w.spawn_x, w.spawn_y);
                    }
                }

                if let Some((_, t)) = map_banner.as_mut() {
                    *t -= dt;
                    if *t <= 0.0 {
                        map_banner = None;
                    }
                }

                let encounter_triggered = player.as_ref().map(|p| p.trigger_encounter).unwrap_or(false);
                if encounter_triggered {
                    if let Some(p) = player.as_mut() {
                        p.trigger_encounter = false;
                    }
                    let wild: Vec<&crate::data::CreatureData> = game_data
                        .creatures
                        .iter()
                        .filter(|c| !c.wild_levels.is_empty())
                        .collect();
                    if !wild.is_empty()
                        && let Some(pc) = player_creature.as_ref() {
                            let mut rng = ::rand::thread_rng();
                            let cd = wild[rng.gen_range(0..wild.len())];
                            let min_lv = cd.wild_levels[0] as u8;
                            let max_lv = *cd.wild_levels.last().expect("wild_levels vide") as u8;
                            let enemy_lv = rng.gen_range(min_lv..=max_lv);
                            let enemy = CreatureInstance::from_data(cd, enemy_lv);
                            state = GameState::CombatWild {
                                combat: CombatState::new(pc.clone(), enemy, false, None),
                            };
                        }
                }

                if is_key_pressed(KeyCode::I) {
                    state = GameState::InventoryOverlay { selected: 0 };
                }

                if is_key_pressed(KeyCode::F5)
                    && let (Some(pr), Some(p), Some(pc)) = (profile.as_ref(), player.as_ref(), player_creature.as_ref()) {
                        SaveData::from_game_state(
                            &pr.name,
                            p.tile_x,
                            p.tile_y,
                            &current_map,
                            pc,
                            &inventory.items,
                            &defeated_trainers,
                        ).save("save.json");
                        save_flash = 1.5;
                    }

                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::PauseMenu { selected: 0 };
                }

                if is_key_pressed(KeyCode::Space)
                    && let Some(p) = player.as_ref() {
                        let (fx, fy) = facing_tile(p);
                        if let Some(idx) = npcs
                            .iter()
                            .position(|n| n.data.tile_x == fx && n.data.tile_y == fy)
                        {
                            npcs[idx].start_dialogue();
                            state = GameState::Dialogue { npc_index: idx };
                        }
                    }
            }

            GameState::InventoryOverlay { selected } => {
                let ids = inventory.ordered_ids();
                let n   = ids.len();
                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::Explore;
                } else {
                    if (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S))
                        && *selected + 1 < n { *selected += 1; }
                    if (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W))
                        && *selected > 0 { *selected -= 1; }
                    if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
                        && n > 0 {
                            let item_id = ids[*selected].to_string();
                            let effect  = game_data.items.iter()
                                .find(|d| d.id == item_id)
                                .map(|d| d.effect.as_str()).unwrap_or("");
                            let val     = game_data.items.iter()
                                .find(|d| d.id == item_id)
                                .map(|d| d.value as u32).unwrap_or(0);
                            if effect == "heal"
                                && inventory.use_item(&item_id)
                                    && let Some(pc) = player_creature.as_mut() {
                                        pc.current_hp = (pc.current_hp + val).min(pc.max_hp);
                                    }
                        }
                }
            }

            GameState::PauseMenu { selected } => {
                let sel = *selected;
                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::Explore;
                } else if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Z) {
                    if sel > 0 { *selected = sel - 1; }
                } else if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    if sel + 1 < 5 { *selected = sel + 1; }
                } else if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
                    match sel {
                        0 => state = GameState::PauseInventory { selected: 0 },
                        1 => state = GameState::PauseCharacter,
                        2 => state = GameState::PauseMap,
                        3 => pause_save_requested = true,
                        4 => state = GameState::PauseSettings { selected: 0, confirm_delete: false },
                        _ => {}
                    }
                }
            }

            GameState::PauseInventory { selected } => {
                let ids = inventory.ordered_ids();
                let n   = ids.len();
                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::PauseMenu { selected: 0 };
                } else {
                    if (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S))
                        && *selected + 1 < n { *selected += 1; }
                    if (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W))
                        && *selected > 0 { *selected -= 1; }
                    if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter))
                        && n > 0 {
                            let item_id = ids[*selected].to_string();
                            let effect  = game_data.items.iter()
                                .find(|d| d.id == item_id)
                                .map(|d| d.effect.as_str()).unwrap_or("");
                            let val     = game_data.items.iter()
                                .find(|d| d.id == item_id)
                                .map(|d| d.value as u32).unwrap_or(0);
                            if effect == "heal"
                                && inventory.use_item(&item_id)
                                    && let Some(pc) = player_creature.as_mut() {
                                        pc.current_hp = (pc.current_hp + val).min(pc.max_hp);
                                    }
                        }
                }
            }

            GameState::PauseCharacter => {
                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::PauseMenu { selected: 1 };
                }
            }

            GameState::PauseMap => {
                if is_key_pressed(KeyCode::Escape) {
                    state = GameState::PauseMenu { selected: 2 };
                }
            }

            GameState::PauseSettings { selected, confirm_delete } => {
                let sel  = *selected;
                let conf = *confirm_delete;
                if conf {
                    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Y) {
                        pause_delete_save_requested = true;
                    } else if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::N) {
                        *confirm_delete = false;
                    }
                } else if is_key_pressed(KeyCode::Escape) {
                    state = GameState::PauseMenu { selected: 4 };
                } else if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Z) {
                    if sel > 0 { *selected = sel - 1; }
                } else if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    if sel + 1 < 2 { *selected = sel + 1; }
                } else if (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space))
                    && sel == 1 { *confirm_delete = true; }
            }

            GameState::Dialogue { npc_index } => {
                let idx = *npc_index;
                if is_key_pressed(KeyCode::Escape) {
                    npcs[idx].is_talking = false;
                    state = GameState::Explore;
                } else if is_key_pressed(KeyCode::Space) {
                    match npcs[idx].advance_dialogue() {
                        DialogueResult::Continue => {}
                        DialogueResult::Done => {
                            state = GameState::Explore;
                        }
                        DialogueResult::TriggerBattle => {
                            let npc_data = &npcs[idx].data;
                            let trainer_id = npc_data.id.clone();
                            let trainer_name = npc_data.name.clone();
                            let defeat_quote = npc_data.dialogue_defeated
                                .as_ref()
                                .and_then(|dl| dl.first())
                                .cloned();

                            if let Some(team) = &npc_data.trainer_team.clone()
                                && let (Some(tc), Some(pc)) = (team.first(), player_creature.as_ref()) {
                                    let cd = game_data.get_creature(&tc.creature_id);
                                    let enemy = CreatureInstance::from_data(cd, tc.level);
                                    let mut combat = CombatState::new(
                                        pc.clone(), enemy, true, Some(trainer_name),
                                    );
                                    combat.trainer_defeat_quote = defeat_quote;
                                    state = GameState::CombatTrainer { trainer_id, combat };
                                }
                        }
                    }
                }
            }

            GameState::CombatWild { combat } => {
                match combat.handle_input(&game_data.attacks, &game_data.items, &mut inventory) {
                    CombatOutcome::Exit { final_player_hp, victory } => {
                        combat_exit = Some((final_player_hp, victory));
                    }
                    CombatOutcome::Captured { final_player_hp, captured_name } => {
                        eprintln!("Capturé : {captured_name}");
                        combat_exit = Some((final_player_hp, true));
                    }
                    CombatOutcome::Ongoing => {}
                }
            }

            GameState::CombatTrainer { trainer_id, combat } => {
                match combat.handle_input(&game_data.attacks, &game_data.items, &mut inventory) {
                    CombatOutcome::Exit { final_player_hp, victory } => {
                        combat_exit = Some((final_player_hp, victory));
                        combat_trainer_id = Some(trainer_id.clone());
                    }
                    CombatOutcome::Captured { .. } => {}
                    CombatOutcome::Ongoing => {}
                }
            }

            GameState::GameOver => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    // Retour à Nebular City avec HP max
                    map = match GameMap::load("assets/maps/town.tmx") {
                        Ok(m) => m,
                        Err(e) => { fatal_error(&e).await; return; }
                    };
                    current_map = "town".to_string();
                    npcs = create_npcs(&game_data, "town", &npc_texture);
                    apply_defeated_trainers(&mut npcs, &defeated_trainers);
                    if let Some(p) = player.as_mut() {
                        p.teleport(5, 4);
                    }
                    if let Some(pc) = player_creature.as_mut() {
                        pc.current_hp = pc.max_hp;
                    }
                    state = GameState::Explore;
                }
            }
        }

        // Transition retour Explore après un combat
        if let Some((final_hp, victory)) = combat_exit {
            if let Some(pc) = player_creature.as_mut() {
                pc.current_hp = final_hp;
            }
            if victory {
                if let Some(tid) = combat_trainer_id {
                    defeated_trainers.push(tid.clone());
                    if let Some(npc) = npcs.iter_mut().find(|n| n.data.id == tid) {
                        npc.defeated = true;
                        map_banner = Some((format!("Tu as battu {} !", npc.data.name), 3.0));
                    }
                }
                state = GameState::Explore;
            } else {
                // Défaite → GameOver avec HP max restaurés
                if let Some(pc) = player_creature.as_mut() {
                    pc.current_hp = pc.max_hp;
                }
                state = GameState::GameOver;
            }
        }

        if pause_save_requested {
            if let (Some(pr), Some(p), Some(pc)) = (profile.as_ref(), player.as_ref(), player_creature.as_ref()) {
                SaveData::from_game_state(
                    &pr.name, p.tile_x, p.tile_y, &current_map, pc, &inventory.items, &defeated_trainers,
                ).save("save.json");
                save_flash = 1.5;
            }
            state = GameState::Explore;
        }

        if pause_delete_save_requested {
            let _ = std::fs::remove_file("save.json");
            state = GameState::Explore;
        }

        // --- Rendu ---
        match &state {
            GameState::TitleScreen => {
                set_default_camera();
                clear_background(BLACK);
                draw_title_screen(&logo_texture, save_data.is_some());
            }

            GameState::NameEntry { buffer } => {
                set_default_camera();
                clear_background(Color::from_hex(0x07070f));
                draw_name_entry(buffer);
            }

            GameState::Explore => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs {
                    npc.render();
                }
                if let Some(p) = player.as_ref() {
                    p.render();
                }
                map.render_canopy(Vec2::ZERO);

                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                if let Some((name, _)) = &map_banner {
                    draw_map_banner(name, PHYS_W, PHYS_H);
                }
                if save_flash > 0.0 {
                    let msg = "Sauvegarde effectuée.";
                    let mw = measure_text(msg, None, 22, 1.0).width;
                    draw_text(msg, PHYS_W * 0.5 - mw * 0.5, PHYS_H - 40.0, 22.0, CYAN);
                }
            }

            GameState::Dialogue { npc_index } => {
                let idx = *npc_index;
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs {
                    npc.render();
                }
                if let Some(p) = player.as_ref() {
                    p.render();
                }
                map.render_canopy(Vec2::ZERO);

                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                npcs[idx].render_dialogue_box(PHYS_W, PHYS_H);
            }

            GameState::CombatWild { combat } => {
                set_default_camera();
                combat.render(PHYS_W, PHYS_H, &game_data.attacks, &inventory, &game_data.items);
            }

            GameState::CombatTrainer { combat, .. } => {
                set_default_camera();
                combat.render(PHYS_W, PHYS_H, &game_data.attacks, &inventory, &game_data.items);
            }

            GameState::InventoryOverlay { selected } => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs { npc.render(); }
                if let Some(p) = player.as_ref() { p.render(); }
                map.render_canopy(Vec2::ZERO);
                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                inventory.render_menu(PHYS_W, PHYS_H, *selected, &game_data.items);
            }

            GameState::PauseMenu { selected } => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs { npc.render(); }
                if let Some(p) = player.as_ref() { p.render(); }
                map.render_canopy(Vec2::ZERO);
                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                draw_rectangle(0.0, 0.0, PHYS_W, PHYS_H, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 });
                draw_pause_menu(PHYS_W, PHYS_H, *selected);
            }

            GameState::PauseInventory { selected } => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs { npc.render(); }
                if let Some(p) = player.as_ref() { p.render(); }
                map.render_canopy(Vec2::ZERO);
                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                inventory.render_menu(PHYS_W, PHYS_H, *selected, &game_data.items);
            }

            GameState::PauseCharacter => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs { npc.render(); }
                if let Some(p) = player.as_ref() { p.render(); }
                map.render_canopy(Vec2::ZERO);
                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                let pname = profile.as_ref().map(|p| p.name.as_str()).unwrap_or("?");
                draw_pause_character(PHYS_W, PHYS_H, pname, player_creature.as_ref(), &game_data.attacks);
            }

            GameState::PauseMap => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs { npc.render(); }
                if let Some(p) = player.as_ref() { p.render(); }
                map.render_canopy(Vec2::ZERO);
                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                let (ptx, pty) = player.as_ref().map(|p| (p.tile_x, p.tile_y)).unwrap_or((0, 0));
                draw_pause_map(PHYS_W, PHYS_H, &map, map_display_name(&current_map), ptx, pty);
            }

            GameState::PauseSettings { selected, confirm_delete } => {
                set_camera(&scene_cam);
                clear_background(BLACK);
                map.render(Vec2::ZERO);
                for npc in &npcs { npc.render(); }
                if let Some(p) = player.as_ref() { p.render(); }
                map.render_canopy(Vec2::ZERO);
                set_default_camera();
                clear_background(BLACK);
                blit_target(&target.texture);
                draw_pause_settings(PHYS_W, PHYS_H, *selected, *confirm_delete);
            }

            GameState::GameOver => {
                set_default_camera();
                clear_background(BLACK);
                let msg = "GAME OVER";
                let mw = measure_text(msg, None, 48, 1.0).width;
                draw_text(msg, PHYS_W * 0.5 - mw * 0.5, PHYS_H * 0.45, 48.0, RED);
                let sub = "Votre équipe a été récupérée à Nebular City.";
                let sw = measure_text(sub, None, 18, 1.0).width;
                draw_text(sub, PHYS_W * 0.5 - sw * 0.5, PHYS_H * 0.58, 18.0, WHITE);
                if (get_time() % 1.0) < 0.65 {
                    let hint = "[ Espace ]  Continuer";
                    let hw = measure_text(hint, None, 22, 1.0).width;
                    draw_text(hint, PHYS_W * 0.5 - hw * 0.5, PHYS_H * 0.72, 22.0, CYAN);
                }
            }
        }

        next_frame().await;
    }
}

fn facing_tile(player: &Player) -> (i32, i32) {
    match player.direction {
        Direction::Up    => (player.tile_x, player.tile_y - 1),
        Direction::Down  => (player.tile_x, player.tile_y + 1),
        Direction::Left  => (player.tile_x - 1, player.tile_y),
        Direction::Right => (player.tile_x + 1, player.tile_y),
    }
}

fn blit_target(tex: &Texture2D) {
    draw_texture_ex(
        tex,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(PHYS_W, PHYS_H)),
            flip_y: true,
            ..Default::default()
        },
    );
}

fn draw_map_banner(name: &str, phys_w: f32, _phys_h: f32) {
    let banner_h = 48.0;
    let banner_y = 20.0;
    draw_rectangle(0.0, banner_y, phys_w, banner_h, Color { r: 0.02, g: 0.02, b: 0.08, a: 0.82 });
    draw_rectangle_lines(0.0, banner_y, phys_w, banner_h, 1.5, CYAN);
    let tw = measure_text(name, None, 30, 1.0).width;
    draw_text(name, phys_w * 0.5 - tw * 0.5, banner_y + 32.0, 30.0, CYAN);
}

const CYAN:   Color = Color { r: 0.0,  g: 1.0,  b: 1.0,  a: 1.0 };
const PURPLE: Color = Color { r: 0.27, g: 0.04, b: 0.47, a: 1.0 };
const DIM:    Color = Color { r: 0.22, g: 0.22, b: 0.35, a: 1.0 };
const BORDER: Color = Color { r: 0.0,  g: 0.8,  b: 1.0,  a: 0.55 };

fn map_path_for_id(id: &str) -> &'static str {
    match id {
        "route1" => "assets/maps/route1.tmx",
        _        => "assets/maps/town.tmx",
    }
}

fn apply_defeated_trainers(npcs: &mut [NpcInstance], defeated: &[String]) {
    for npc in npcs.iter_mut() {
        if defeated.contains(&npc.data.id) {
            npc.defeated = true;
        }
    }
}

fn draw_title_screen(logo: &Texture2D, has_save: bool) {
    let img_w = logo.width();
    let img_h = logo.height();
    let scale = PHYS_H / img_h;
    let draw_w = img_w * scale;
    let draw_x = (PHYS_W - draw_w) * 0.5;
    draw_texture_ex(
        logo,
        draw_x, 0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(draw_w, PHYS_H)),
            ..Default::default()
        },
    );

    let overlay_h = 170.0;
    draw_rectangle(0.0, PHYS_H - overlay_h, PHYS_W, overlay_h,
        Color { r: 0.0, g: 0.0, b: 0.02, a: 0.82 });

    let cx = PHYS_W * 0.5;

    if has_save {
        let hint = "[ Sauvegarde detectee ]";
        let hw = measure_text(hint, None, 17, 1.0).width;
        draw_text(hint, cx - hw * 0.5, PHYS_H - 112.0, 17.0,
            Color { r: 0.4, g: 1.0, b: 0.6, a: 1.0 });
    }

    if (get_time() % 1.0) < 0.65 {
        let label = if has_save { "[ Entree ]  Continuer" } else { "[ Entree ]  Nouvelle partie" };
        let lw = measure_text(label, None, 24, 1.0).width;
        draw_text(label, cx - lw * 0.5, PHYS_H - 72.0, 24.0, CYAN);
    }

    let quit = "[ Echap ]  Quitter";
    let qw = measure_text(quit, None, 15, 1.0).width;
    draw_text(quit, cx - qw * 0.5, PHYS_H - 38.0, 15.0, DIM);

    draw_text("v0.1-alpha", PHYS_W - 90.0, 22.0, 14.0, DIM);
}

fn map_display_name(id: &str) -> &'static str {
    match id {
        "route1" => "Wiki Road",
        _        => "Nebular City",
    }
}

fn draw_pause_menu(phys_w: f32, phys_h: f32, selected: usize) {
    const ITEMS: &[&str] = &["Inventaire", "Personnage", "Carte", "Sauvegarder", "Parametres"];
    let box_w = 300.0;
    let row_h = 50.0;
    let box_h = 56.0 + ITEMS.len() as f32 * row_h + 20.0;
    let box_x = phys_w * 0.5 - box_w * 0.5;
    let box_y = phys_h * 0.5 - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.02, g: 0.04, b: 0.10, a: 1.0 });
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

    let title = "MENU";
    let tw = measure_text(title, None, 26, 1.0).width;
    draw_text(title, phys_w * 0.5 - tw * 0.5, box_y + 34.0, 26.0, CYAN);
    draw_line(box_x + 12.0, box_y + 44.0, box_x + box_w - 12.0, box_y + 44.0, 1.0, CYAN);

    for (i, &item) in ITEMS.iter().enumerate() {
        let cy = box_y + 52.0 + i as f32 * row_h;
        let is_sel = i == selected;
        if is_sel {
            draw_rectangle(box_x + 4.0, cy, box_w - 8.0, row_h - 4.0,
                Color { r: 0.0, g: 0.22, b: 0.22, a: 1.0 });
            draw_rectangle_lines(box_x + 4.0, cy, box_w - 8.0, row_h - 4.0, 1.5, CYAN);
        }
        let col = if is_sel { CYAN } else { WHITE };
        let label = if is_sel { format!("> {}", item) } else { format!("  {}", item) };
        draw_text(&label, box_x + 22.0, cy + row_h * 0.65, 24.0, col);
    }

    let hint = "[ Z/S ]  naviguer   [ Entree ]  valider   [ Echap ]  fermer";
    let hw = measure_text(hint, None, 12, 1.0).width;
    draw_text(hint, phys_w * 0.5 - hw * 0.5, box_y + box_h - 8.0, 12.0, DIM);
}

fn draw_pause_character(phys_w: f32, phys_h: f32, player_name: &str,
                         creature: Option<&crate::creature::CreatureInstance>,
                         attacks: &[AttackData]) {
    draw_rectangle(0.0, 0.0, phys_w, phys_h, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.88 });

    let box_w = 500.0;
    let box_h = 390.0;
    let box_x = phys_w * 0.5 - box_w * 0.5;
    let box_y = phys_h * 0.5 - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.02, g: 0.04, b: 0.10, a: 1.0 });
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

    let title = "PERSONNAGE";
    let tw = measure_text(title, None, 26, 1.0).width;
    draw_text(title, phys_w * 0.5 - tw * 0.5, box_y + 34.0, 26.0, CYAN);
    draw_line(box_x + 12.0, box_y + 44.0, box_x + box_w - 12.0, box_y + 44.0, 1.0, CYAN);

    draw_text(&format!("Dresseur : {}", player_name), box_x + 20.0, box_y + 70.0, 20.0, WHITE);

    let Some(c) = creature else {
        draw_text("Aucune creature.", box_x + 20.0, box_y + 110.0, 22.0, DIM);
        let hint = "[ Echap ] retour";
        let hw = measure_text(hint, None, 14, 1.0).width;
        draw_text(hint, phys_w * 0.5 - hw * 0.5, box_y + box_h - 10.0, 14.0, DIM);
        return;
    };

    draw_text(&c.name, box_x + 20.0, box_y + 108.0, 28.0, CYAN);
    let lv_str = format!("Niv. {}", c.level);
    let lw = measure_text(&lv_str, None, 22, 1.0).width;
    draw_text(&lv_str, box_x + box_w - lw - 20.0, box_y + 108.0, 22.0, WHITE);

    let type_str = c.creature_type.join(" / ");
    draw_text(&format!("Type : {}", type_str), box_x + 20.0, box_y + 134.0, 17.0, DIM);

    let bar_x = box_x + 20.0;
    let bar_y = box_y + 152.0;
    let bar_w = box_w - 40.0;
    let bar_h = 20.0;
    let ratio  = c.current_hp as f32 / c.max_hp as f32;
    let hp_col = if ratio > 0.5 {
        Color { r: 0.1, g: 0.85, b: 0.3,  a: 1.0 }
    } else if ratio > 0.2 {
        Color { r: 0.9, g: 0.8,  b: 0.1,  a: 1.0 }
    } else {
        Color { r: 0.9, g: 0.15, b: 0.15, a: 1.0 }
    };
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 });
    draw_rectangle(bar_x, bar_y, bar_w * ratio, bar_h, hp_col);
    draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 1.0, CYAN);
    let hp_str = format!("PV : {} / {}", c.current_hp, c.max_hp);
    let hlw = measure_text(&hp_str, None, 15, 1.0).width;
    draw_text(&hp_str, bar_x + bar_w * 0.5 - hlw * 0.5, bar_y + 14.0, 15.0, WHITE);

    let sy = box_y + 192.0;
    let col_w = (box_w - 40.0) / 3.0;
    for (i, (label, val)) in [("ATQ", c.attack), ("DEF", c.defense), ("VIT", c.speed)].iter().enumerate() {
        let sx = box_x + 20.0 + i as f32 * col_w;
        draw_text(label, sx, sy, 17.0, DIM);
        draw_text(&val.to_string(), sx, sy + 26.0, 28.0, WHITE);
    }

    draw_line(box_x + 12.0, sy + 44.0, box_x + box_w - 12.0, sy + 44.0, 1.0, BORDER);
    draw_text("Attaques :", box_x + 20.0, sy + 64.0, 17.0, DIM);
    for (i, move_id) in c.moves.iter().enumerate() {
        let atk = attacks.iter().find(|a| a.id == *move_id);
        let name = atk.map(|a| a.name.as_str()).unwrap_or(move_id.as_str());
        let power_info = atk.map(|a| {
            if a.power > 0 { format!(" ({} dmg)", a.power) } else { " (statut)".to_string() }
        }).unwrap_or_default();
        let mx = box_x + 20.0 + (i % 2) as f32 * (box_w * 0.5 - 10.0);
        let my = sy + 86.0 + (i / 2) as f32 * 30.0;
        draw_text(&format!("• {}{}", name, power_info), mx, my, 18.0, WHITE);
    }

    let hint = "[ Echap ] retour";
    let hw = measure_text(hint, None, 14, 1.0).width;
    draw_text(hint, phys_w * 0.5 - hw * 0.5, box_y + box_h - 10.0, 14.0, DIM);
}

fn draw_pause_map(phys_w: f32, phys_h: f32, map: &GameMap,
                  map_name: &str, player_tx: i32, player_ty: i32) {
    draw_rectangle(0.0, 0.0, phys_w, phys_h, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.88 });

    let box_w = 500.0;
    let box_h = 430.0;
    let box_x = phys_w * 0.5 - box_w * 0.5;
    let box_y = phys_h * 0.5 - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.02, g: 0.04, b: 0.10, a: 1.0 });
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

    let tw = measure_text(map_name, None, 26, 1.0).width;
    draw_text(map_name, phys_w * 0.5 - tw * 0.5, box_y + 34.0, 26.0, CYAN);
    draw_line(box_x + 12.0, box_y + 44.0, box_x + box_w - 12.0, box_y + 44.0, 1.0, CYAN);

    let mm_x = box_x + 20.0;
    let mm_y = box_y + 56.0;
    let mm_w = box_w - 40.0;
    let mm_h = box_h - 56.0 - 68.0;
    map.render_minimap(mm_x, mm_y, mm_w, mm_h, player_tx, player_ty);
    draw_rectangle_lines(mm_x, mm_y, mm_w, mm_h, 1.5, CYAN);

    let leg_y = mm_y + mm_h + 14.0;
    draw_rectangle(box_x + 20.0, leg_y, 14.0, 14.0,
        Color { r: 0.06, g: 0.06, b: 0.10, a: 1.0 });
    draw_text("Obstacle", box_x + 40.0, leg_y + 12.0, 14.0, DIM);

    draw_rectangle(box_x + 140.0, leg_y, 14.0, 14.0,
        Color { r: 0.08, g: 0.28, b: 0.12, a: 1.0 });
    draw_text("Zone rencontre", box_x + 160.0, leg_y + 12.0, 14.0, DIM);

    draw_circle(box_x + 304.0, leg_y + 7.0, 5.0, CYAN);
    draw_text("Joueur", box_x + 316.0, leg_y + 12.0, 14.0, DIM);

    let hint = "[ Echap ] retour";
    let hw = measure_text(hint, None, 14, 1.0).width;
    draw_text(hint, phys_w * 0.5 - hw * 0.5, box_y + box_h - 10.0, 14.0, DIM);
}

fn draw_pause_settings(phys_w: f32, phys_h: f32, selected: usize, confirm_delete: bool) {
    draw_rectangle(0.0, 0.0, phys_w, phys_h, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.88 });

    let box_w = 560.0;
    let box_h = 390.0;
    let box_x = phys_w * 0.5 - box_w * 0.5;
    let box_y = phys_h * 0.5 - box_h * 0.5;

    draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.02, g: 0.04, b: 0.10, a: 1.0 });
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

    let title = "PARAMETRES";
    let tw = measure_text(title, None, 26, 1.0).width;
    draw_text(title, phys_w * 0.5 - tw * 0.5, box_y + 34.0, 26.0, CYAN);
    draw_line(box_x + 12.0, box_y + 44.0, box_x + box_w - 12.0, box_y + 44.0, 1.0, CYAN);

    const OPTS: &[&str] = &["Controles", "Supprimer save"];
    let opt_col_w = 190.0;
    let row_h = 50.0;
    for (i, &opt) in OPTS.iter().enumerate() {
        let oy = box_y + 54.0 + i as f32 * row_h;
        let is_sel = i == selected;
        if is_sel {
            draw_rectangle(box_x + 4.0, oy, opt_col_w, row_h - 4.0,
                Color { r: 0.0, g: 0.22, b: 0.22, a: 1.0 });
            draw_rectangle_lines(box_x + 4.0, oy, opt_col_w, row_h - 4.0, 1.5, CYAN);
        }
        let danger = Color { r: 0.9, g: 0.2, b: 0.2, a: 1.0 };
        let col = if is_sel { CYAN } else if i == 1 { danger } else { WHITE };
        draw_text(opt, box_x + 14.0, oy + row_h * 0.65, 18.0, col);
    }

    let sep_x = box_x + opt_col_w + 12.0;
    draw_line(sep_x, box_y + 52.0, sep_x, box_y + box_h - 40.0, 1.0, BORDER);

    let cx = sep_x + 16.0;
    let cy = box_y + 62.0;

    if confirm_delete {
        draw_text("Supprimer la sauvegarde ?", cx, cy, 18.0,
            Color { r: 0.9, g: 0.2, b: 0.2, a: 1.0 });
        draw_text("Cette action est irreversible.", cx, cy + 28.0, 15.0, DIM);
        if (get_time() % 1.0) < 0.65 {
            draw_text("[ Entree ] Confirmer   [ Echap ] Annuler", cx, cy + 62.0, 15.0, CYAN);
        }
    } else if selected == 0 {
        const BINDS: &[(&str, &str)] = &[
            ("Z / Haut",         "Monter"),
            ("S / Bas",          "Descendre"),
            ("Q / Gauche",       "Gauche"),
            ("D / Droite",       "Droite"),
            ("Espace",           "Interagir / Attaque"),
            ("I",                "Inventaire (exploration)"),
            ("F5",               "Sauvegarder"),
            ("Echap",            "Menu pause / Retour"),
        ];
        for (i, (key, action)) in BINDS.iter().enumerate() {
            let by = cy + i as f32 * 28.0;
            draw_text(key,    cx,          by, 14.0, CYAN);
            draw_text(action, cx + 160.0, by, 14.0, WHITE);
        }
    } else {
        draw_text("Supprime definitivement votre", cx, cy, 15.0, DIM);
        draw_text("fichier de sauvegarde.", cx, cy + 22.0, 15.0, DIM);
        draw_text("[ Entree ] pour confirmer.", cx, cy + 54.0, 15.0,
            Color { r: 0.9, g: 0.2, b: 0.2, a: 1.0 });
    }

    let hint = "[ Z/S ] naviguer   [ Echap ] retour";
    let hw = measure_text(hint, None, 13, 1.0).width;
    draw_text(hint, phys_w * 0.5 - hw * 0.5, box_y + box_h - 10.0, 13.0, DIM);
}

fn draw_name_entry(buffer: &str) {
    let cx = PHYS_W * 0.5;

    let margin = 24.0;
    draw_rectangle_lines(margin, margin, PHYS_W - margin * 2.0, PHYS_H - margin * 2.0, 2.0, BORDER);

    for (x, y) in [(margin, margin), (PHYS_W - margin, margin),
                   (margin, PHYS_H - margin), (PHYS_W - margin, PHYS_H - margin)] {
        draw_text("+", x - 6.0, y + 6.0, 18.0, CYAN);
    }

    let title_agi = "AGI";
    let tw_agi = measure_text(title_agi, None, 80, 1.0).width;
    draw_text(title_agi, cx - tw_agi * 0.5, 140.0, 80.0, CYAN);

    let title_adv = "ADVENTURE";
    let tw_adv = measure_text(title_adv, None, 38, 1.0).width;
    draw_text(title_adv, cx - tw_adv * 0.5, 180.0, 38.0, WHITE);

    let lx = cx - 140.0;
    draw_line(lx, 196.0, cx - 10.0, 196.0, 1.5, BORDER);
    draw_rectangle(cx - 6.0, 192.0, 12.0, 5.0, PURPLE);
    draw_line(cx + 10.0, 196.0, lx + 280.0, 196.0, 1.5, BORDER);

    let prompt = "Quel est ton nom, hacker ?";
    let pw = measure_text(prompt, None, 22, 1.0).width;
    draw_text(prompt, cx - pw * 0.5, 266.0, 22.0, WHITE);

    let box_w = 260.0;
    let box_h = 46.0;
    let box_x = cx - box_w * 0.5;
    let box_y = 288.0;
    draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.0, g: 0.08, b: 0.15, a: 1.0 });
    draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

    let cursor = if (get_time() % 0.8) < 0.45 { "▌" } else { " " };
    let input_str = format!("{}{}", buffer, cursor);
    let iw = measure_text(&input_str, None, 28, 1.0).width;
    draw_text(&input_str, cx - iw * 0.5, box_y + 32.0, 28.0, CYAN);

    let hint = "[ Enter ]  pour confirmer";
    let hw = measure_text(hint, None, 16, 1.0).width;
    draw_text(hint, cx - hw * 0.5, 390.0, 16.0, DIM);

    draw_text("v0.1-alpha", PHYS_W - 90.0, PHYS_H - 34.0, 14.0, DIM);
}

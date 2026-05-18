use macroquad::prelude::*;
use ::rand::Rng;

use crate::creature::CreatureInstance;
use crate::data::{AttackData, ItemData};
use crate::inventory::Inventory;

// ── Constantes de layout (coordonnées physiques 640×576) ──────────────────
const SCENE_H: f32  = 300.0;
const LOG_H: f32    = 70.0;
const MENU_Y: f32   = 370.0;
const BOX_SZ: f32   = 96.0;

// ── Palette ───────────────────────────────────────────────────────────────
const CYAN:    Color = Color { r: 0.0,  g: 1.0,  b: 1.0,  a: 1.0 };
const BG:      Color = Color { r: 0.04, g: 0.10, b: 0.04, a: 1.0 };
const LOG_BG:  Color = Color { r: 0.02, g: 0.06, b: 0.02, a: 1.0 };
const MENU_BG: Color = Color { r: 0.01, g: 0.04, b: 0.01, a: 1.0 };
const SEL_HL:  Color = Color { r: 0.0,  g: 0.22, b: 0.22, a: 1.0 };
const PP_COL:  Color = Color { r: 0.5,  g: 0.75, b: 0.5,  a: 1.0 };
const DIM:     Color = Color { r: 0.3,  g: 0.3,  b: 0.3,  a: 1.0 };
const GOLD:    Color = Color { r: 0.9,  g: 0.75, b: 0.1,  a: 1.0 };
const RED_ERR: Color = Color { r: 0.9,  g: 0.15, b: 0.1,  a: 1.0 };

// ── Types publics ─────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq)]
pub enum TurnPhase {
    PlayerChoose,
    ItemMenu { selected: usize },
    ShowResult,
    Victory,
    Defeat,
}

#[derive(Clone)]
pub enum CombatOutcome {
    Ongoing,
    Exit { final_player_hp: u32, victory: bool },
    Captured { final_player_hp: u32, captured_name: String },
}

pub struct CombatState {
    pub player_creature: CreatureInstance,
    pub enemy_creature: CreatureInstance,
    pub is_trainer_battle: bool,
    pub trainer_name: Option<String>,
    pub trainer_defeat_quote: Option<String>,
    pub turn: TurnPhase,
    pub log: Vec<String>,
    pub selected_move: usize,
}

impl CombatState {
    pub fn new(
        player: CreatureInstance,
        enemy: CreatureInstance,
        is_trainer: bool,
        trainer_name: Option<String>,
    ) -> CombatState {
        CombatState {
            player_creature: player,
            enemy_creature: enemy,
            is_trainer_battle: is_trainer,
            trainer_name,
            trainer_defeat_quote: None,
            turn: TurnPhase::PlayerChoose,
            log: Vec::new(),
            selected_move: 0,
        }
    }

    // ── Input ─────────────────────────────────────────────────────────────

    pub fn handle_input(
        &mut self,
        attacks: &[AttackData],
        items: &[ItemData],
        inventory: &mut Inventory,
    ) -> CombatOutcome {
        let n_moves = self.player_creature.moves.len();
        let bag_available = n_moves < 4;
        // Nombre de slots navigables dans la grille (moves + Bag si dispo)
        let effective_n = if bag_available { n_moves + 1 } else { n_moves };
        // Bag occupe le slot juste après le dernier move
        let bag_slot = n_moves;

        match self.turn {
            TurnPhase::PlayerChoose => {
                if effective_n > 0 {
                    if (is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D))
                        && self.selected_move.is_multiple_of(2) && self.selected_move + 1 < effective_n {
                            self.selected_move += 1;
                        }
                    if (is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A))
                        && self.selected_move % 2 == 1 {
                            self.selected_move -= 1;
                        }
                    if (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S))
                        && self.selected_move + 2 < effective_n {
                            self.selected_move += 2;
                        }
                    if (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W))
                        && self.selected_move >= 2 {
                            self.selected_move -= 2;
                        }
                    if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                        if bag_available && self.selected_move == bag_slot {
                            self.turn = TurnPhase::ItemMenu { selected: 0 };
                        } else {
                            self.execute_turn(attacks);
                        }
                    }
                }
            }

            TurnPhase::ItemMenu { selected } => {
                let item_ids = inventory.ordered_ids();
                let item_count = item_ids.len();
                let mut sel = selected;

                if is_key_pressed(KeyCode::Escape) {
                    self.turn = TurnPhase::PlayerChoose;
                    return CombatOutcome::Ongoing;
                }
                if (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S))
                    && sel + 1 < item_count { sel += 1; }
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    sel = sel.saturating_sub(1);
                }

                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    if item_count == 0 {
                        self.turn = TurnPhase::PlayerChoose;
                        return CombatOutcome::Ongoing;
                    }
                    let item_id = item_ids[sel].to_string();
                    let effect  = items.iter().find(|d| d.id == item_id)
                        .map(|d| d.effect.as_str()).unwrap_or("");
                    let val     = items.iter().find(|d| d.id == item_id)
                        .map(|d| d.value).unwrap_or(0);

                    if effect == "capture" && self.is_trainer_battle {
                        self.log.push("Ça ne fonctionne pas en combat de dresseur !".to_string());
                        self.turn = TurnPhase::PlayerChoose;
                        return CombatOutcome::Ongoing;
                    }

                    if inventory.use_item(&item_id) {
                        let player_name = self.player_creature.name.clone();
                        let enemy_name  = self.enemy_creature.name.clone();

                        match effect {
                            "heal" => {
                                let healed = (self.player_creature.current_hp + val as u32)
                                    .min(self.player_creature.max_hp);
                                self.player_creature.current_hp = healed;
                                self.log.push(format!(
                                    "Patch appliqué ! {} récupère {} HP.", player_name, val
                                ));
                                let n_enemy = self.enemy_creature.moves.len();
                                if n_enemy > 0 {
                                    let emove = self.enemy_creature.moves
                                        [::rand::thread_rng().gen_range(0..n_enemy)].clone();
                                    if self.apply_attack(false, &emove, attacks) {
                                        self.log.push(format!("{} est K.O. !", player_name));
                                        self.turn = TurnPhase::Defeat;
                                        return CombatOutcome::Ongoing;
                                    }
                                }
                                self.turn = TurnPhase::ShowResult;
                            }

                            "capture" => {
                                let catch_rate  = self.enemy_creature.catch_rate as f32;
                                let catch_chance = catch_rate / 255.0;
                                let roll: f32 = ::rand::random::<f32>();
                                if roll < catch_chance {
                                    self.log.push(format!(
                                        "CTF réussi ! Tu as capturé {} !", enemy_name
                                    ));
                                    return CombatOutcome::Captured {
                                        final_player_hp: self.player_creature.current_hp,
                                        captured_name: enemy_name,
                                    };
                                } else {
                                    self.log.push(format!(
                                        "CTF échoué ! {} s'est échappé !", enemy_name
                                    ));
                                    let n_enemy = self.enemy_creature.moves.len();
                                    if n_enemy > 0 {
                                        let emove  = self.enemy_creature.moves
                                            [::rand::thread_rng().gen_range(0..n_enemy)].clone();
                                        if self.apply_attack(false, &emove, attacks) {
                                            self.log.push(format!("{} est K.O. !", player_name));
                                            self.turn = TurnPhase::Defeat;
                                            return CombatOutcome::Ongoing;
                                        }
                                    }
                                    self.turn = TurnPhase::ShowResult;
                                }
                            }

                            _ => {
                                self.turn = TurnPhase::PlayerChoose;
                            }
                        }
                    } else {
                        let name = items.iter().find(|d| d.id == item_id)
                            .map(|d| d.name.as_str()).unwrap_or(&item_id);
                        self.log.push(format!("Plus de {} !", name));
                        self.turn = TurnPhase::PlayerChoose;
                    }
                    return CombatOutcome::Ongoing;
                }

                // Réécriture du selected après navigation
                self.turn = TurnPhase::ItemMenu { selected: sel };
            }

            TurnPhase::ShowResult => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    self.turn = TurnPhase::PlayerChoose;
                }
            }

            TurnPhase::Victory => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    return CombatOutcome::Exit {
                        final_player_hp: self.player_creature.current_hp,
                        victory: true,
                    };
                }
            }

            TurnPhase::Defeat => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    return CombatOutcome::Exit {
                        final_player_hp: self.player_creature.max_hp / 2,
                        victory: false,
                    };
                }
            }
        }

        CombatOutcome::Ongoing
    }

    // ── Logique de tour ───────────────────────────────────────────────────

    fn execute_turn(&mut self, attacks: &[AttackData]) {
        let n_enemy = self.enemy_creature.moves.len();
        if self.player_creature.moves.is_empty() || n_enemy == 0 {
            return;
        }

        // Pre-clone pour éviter les conflits d'emprunt dans les logs
        let player_name = self.player_creature.name.clone();
        let enemy_name  = self.enemy_creature.name.clone();

        let player_move = self.player_creature.moves[self.selected_move].clone();
        let enemy_move = self.enemy_creature.moves[::rand::thread_rng().gen_range(0..n_enemy)].clone();

        // Ordre d'initiative
        let eff_enemy_speed = if self.enemy_creature.speed_down_turns > 0 {
            self.enemy_creature.speed * 70 / 100
        } else {
            self.enemy_creature.speed
        };
        let player_first = self.player_creature.speed >= eff_enemy_speed;

        let (first_by_player, first_move, second_by_player, second_move) = if player_first {
            (true, player_move.clone(), false, enemy_move.clone())
        } else {
            (false, enemy_move.clone(), true, player_move.clone())
        };

        // Premier attaquant
        if self.apply_attack(first_by_player, &first_move, attacks) {
            let ko = if first_by_player { &enemy_name } else { &player_name };
            self.log.push(format!("{} est K.O. !", ko));
            self.turn = if first_by_player { TurnPhase::Victory } else { TurnPhase::Defeat };
            return;
        }

        // Deuxième attaquant
        if self.apply_attack(second_by_player, &second_move, attacks) {
            let ko = if second_by_player { &enemy_name } else { &player_name };
            self.log.push(format!("{} est K.O. !", ko));
            self.turn = if second_by_player { TurnPhase::Victory } else { TurnPhase::Defeat };
            return;
        }

        // Décrémenter les compteurs de buffs des deux créatures
        if self.player_creature.defense_boost_turns > 0 { self.player_creature.defense_boost_turns -= 1; }
        if self.player_creature.speed_down_turns > 0    { self.player_creature.speed_down_turns -= 1; }
        if self.enemy_creature.defense_boost_turns > 0  { self.enemy_creature.defense_boost_turns -= 1; }
        if self.enemy_creature.speed_down_turns > 0     { self.enemy_creature.speed_down_turns -= 1; }

        self.turn = TurnPhase::ShowResult;
    }

    // Retourne true si la cible est K.O.
    fn apply_attack(&mut self, player_attacks: bool, move_id: &str, attacks: &[AttackData]) -> bool {
        let (atk_name, def_name, atk_power_stat, def_power_stat, def_boost) = if player_attacks {
            (
                self.player_creature.name.clone(),
                self.enemy_creature.name.clone(),
                self.player_creature.attack,
                self.enemy_creature.defense,
                self.enemy_creature.defense_boost_turns,
            )
        } else {
            (
                self.enemy_creature.name.clone(),
                self.player_creature.name.clone(),
                self.enemy_creature.attack,
                self.player_creature.defense,
                self.player_creature.defense_boost_turns,
            )
        };

        let atk_data = attacks.iter().find(|a| a.id == move_id);
        let move_name = atk_data.map(|a| a.name.as_str()).unwrap_or(move_id);

        self.log.push(format!("{} utilise {} !", atk_name, move_name));

        if let Some(atk) = atk_data {
            if atk.power > 0 {
                let damage = calculate_damage(atk_power_stat, atk.power, def_power_stat, def_boost > 0);

                if player_attacks {
                    self.enemy_creature.current_hp = self.enemy_creature.current_hp.saturating_sub(damage);
                } else {
                    self.player_creature.current_hp = self.player_creature.current_hp.saturating_sub(damage);
                }
                self.log.push(format!("{} perd {} PV !", def_name, damage));
            }

            // Effets de statut
            if let Some(effect) = &atk.effect.clone() {
                let duration = atk.effect_duration.unwrap_or(3) as u8;
                match effect.as_str() {
                    "defense_boost" => {
                        if player_attacks {
                            self.player_creature.defense_boost_turns = duration;
                        } else {
                            self.enemy_creature.defense_boost_turns = duration;
                        }
                        self.log.push(format!("{} active son Guardrail !", atk_name));
                    }
                    "speed_down" => {
                        if player_attacks {
                            self.enemy_creature.speed_down_turns = duration;
                        } else {
                            self.player_creature.speed_down_turns = duration;
                        }
                        self.log.push(format!("{} est ralenti par Trend Effect !", def_name));
                    }
                    _ => {}
                }
            }
        }

        // KO ?
        if player_attacks {
            self.enemy_creature.current_hp == 0
        } else {
            self.player_creature.current_hp == 0
        }
    }

    // ── Rendu ─────────────────────────────────────────────────────────────

    pub fn render(&self, phys_w: f32, phys_h: f32, attacks: &[AttackData], inventory: &Inventory, items: &[ItemData]) {
        let menu_h = phys_h - MENU_Y;

        // Fond
        draw_rectangle(0.0, 0.0, phys_w, phys_h, BG);

        // Bandeau "COMBAT DE DRESSEUR" pour les combats trainer
        if self.is_trainer_battle {
            let label = "COMBAT DE DRESSEUR";
            let lw = measure_text(label, None, 22, 1.0).width;
            draw_text(label, phys_w * 0.5 - lw * 0.5, 18.0, 22.0,
                Color { r: 1.0, g: 0.27, b: 0.27, a: 1.0 });
        }

        // ── Panneau ennemi (haut gauche) ─────────────────────────────────
        let eb_x = 24.0;
        let eb_y = 32.0;
        let e_type = self.enemy_creature.creature_type.first().map(|s| s.as_str()).unwrap_or("");
        draw_rectangle(eb_x, eb_y, BOX_SZ, BOX_SZ, type_color(e_type));
        draw_rectangle_lines(eb_x, eb_y, BOX_SZ, BOX_SZ, 2.0, CYAN);

        let ei_x = eb_x + BOX_SZ + 20.0;
        draw_text(&self.enemy_creature.name, ei_x, eb_y + 28.0, 28.0, WHITE);
        draw_text(&format!("Lv.{}", self.enemy_creature.level), ei_x, eb_y + 56.0, 22.0, CYAN);
        draw_text(
            &format!("HP  {}/{}", self.enemy_creature.current_hp, self.enemy_creature.max_hp),
            ei_x, eb_y + 82.0, 20.0, WHITE,
        );
        if self.enemy_creature.defense_boost_turns > 0 {
            draw_text(&format!("[GUARD {}t]", self.enemy_creature.defense_boost_turns), ei_x, eb_y + 104.0, 16.0, CYAN);
        }
        if self.enemy_creature.speed_down_turns > 0 {
            draw_text(&format!("[LENT {}t]", self.enemy_creature.speed_down_turns), ei_x + 90.0, eb_y + 104.0, 16.0, GOLD);
        }
        draw_hp_bar(eb_x, eb_y + BOX_SZ + 10.0, 280.0, 12.0,
            self.enemy_creature.current_hp, self.enemy_creature.max_hp);

        // ── Panneau joueur (bas droite) ───────────────────────────────────
        let pb_x = phys_w - BOX_SZ - 24.0;
        let pb_y = SCENE_H - BOX_SZ - 50.0;
        let p_type = self.player_creature.creature_type.first().map(|s| s.as_str()).unwrap_or("");
        draw_rectangle(pb_x, pb_y, BOX_SZ, BOX_SZ, type_color(p_type));
        draw_rectangle_lines(pb_x, pb_y, BOX_SZ, BOX_SZ, 2.0, CYAN);

        let pi_x = 24.0;
        draw_text(&self.player_creature.name, pi_x, pb_y + 28.0, 28.0, WHITE);
        draw_text(&format!("Lv.{}", self.player_creature.level), pi_x, pb_y + 56.0, 22.0, CYAN);
        draw_text(
            &format!("HP  {}/{}", self.player_creature.current_hp, self.player_creature.max_hp),
            pi_x, pb_y + 82.0, 20.0, WHITE,
        );
        if self.player_creature.defense_boost_turns > 0 {
            draw_text(&format!("[GUARD {}t]", self.player_creature.defense_boost_turns), pi_x, pb_y + 104.0, 16.0, CYAN);
        }
        if self.player_creature.speed_down_turns > 0 {
            draw_text(&format!("[LENT {}t]", self.player_creature.speed_down_turns), pi_x + 90.0, pb_y + 104.0, 16.0, GOLD);
        }
        draw_hp_bar(pi_x, pb_y + BOX_SZ + 10.0, 280.0, 12.0,
            self.player_creature.current_hp, self.player_creature.max_hp);

        // ── Séparateur scène / log ────────────────────────────────────────
        draw_line(0.0, SCENE_H, phys_w, SCENE_H, 1.5, CYAN);

        // ── Zone log — contenu selon la phase ────────────────────────────
        draw_rectangle(0.0, SCENE_H, phys_w, LOG_H, LOG_BG);

        match self.turn {
            TurnPhase::PlayerChoose | TurnPhase::ItemMenu { .. } => {
                if self.log.is_empty() {
                    let intro = if self.is_trainer_battle {
                        format!("{} veut se battre !", self.trainer_name.as_deref().unwrap_or("???"))
                    } else {
                        format!("Un {} sauvage apparaît !", self.enemy_creature.name)
                    };
                    draw_text(&intro, 16.0, SCENE_H + 28.0, 22.0, WHITE);
                    draw_text("Choisis une attaque !", 16.0, SCENE_H + 54.0, 20.0, CYAN);
                } else {
                    self.draw_log_lines(2);
                }
            }

            TurnPhase::ShowResult => {
                self.draw_log_lines(2);
            }

            TurnPhase::Victory => {
                if self.is_trainer_battle {
                    if let Some(quote) = &self.trainer_defeat_quote {
                        let trainer = self.trainer_name.as_deref().unwrap_or("Dresseur");
                        draw_text(
                            &format!("{} : \"{}\"", trainer, quote),
                            16.0, SCENE_H + 32.0, 20.0, GOLD,
                        );
                    } else {
                        draw_text("Victoire ! Dresseur vaincu.", 16.0, SCENE_H + 32.0, 24.0, GOLD);
                    }
                } else {
                    draw_text(
                        &format!("Victoire ! {} est K.O.", self.enemy_creature.name),
                        16.0, SCENE_H + 32.0, 24.0, GOLD,
                    );
                }
            }

            TurnPhase::Defeat => {
                draw_text(
                    &format!("{} est K.O... Patch d'urgence appliqué.", self.player_creature.name),
                    16.0, SCENE_H + 32.0, 22.0, RED_ERR,
                );
            }
        }

        // ── Séparateur log / menu ─────────────────────────────────────────
        draw_line(0.0, MENU_Y, phys_w, MENU_Y, 1.5, CYAN);

        // ── Zone menu — contenu selon la phase ────────────────────────────
        draw_rectangle(0.0, MENU_Y, phys_w, menu_h, MENU_BG);
        draw_rectangle_lines(0.0, MENU_Y, phys_w, menu_h, 1.5, CYAN);

        match self.turn {
            TurnPhase::PlayerChoose | TurnPhase::ItemMenu { .. } => {
                self.draw_move_grid(phys_w, menu_h, attacks);
            }

            TurnPhase::ShowResult => {
                let hint = "[ Espace ]  pour continuer";
                let hw = measure_text(hint, None, 26, 1.0).width;
                draw_text(hint, phys_w * 0.5 - hw * 0.5, MENU_Y + menu_h * 0.55, 26.0, CYAN);
                // indicateur clignotant
                if (get_time() % 0.8) < 0.5 {
                    let arrow = "▼";
                    let aw = measure_text(arrow, None, 22, 1.0).width;
                    draw_text(arrow, phys_w * 0.5 - aw * 0.5, MENU_Y + menu_h * 0.82, 22.0, CYAN);
                }
            }

            TurnPhase::Victory => {
                let lines = [
                    ("EXP gagnée !", GOLD),
                    ("[ Espace ]  pour continuer", CYAN),
                ];
                let line_h = menu_h / (lines.len() as f32 + 1.0);
                for (i, (txt, col)) in lines.iter().enumerate() {
                    let tw = measure_text(txt, None, 26, 1.0).width;
                    draw_text(txt, phys_w * 0.5 - tw * 0.5, MENU_Y + line_h * (i as f32 + 1.2), 26.0, *col);
                }
            }

            TurnPhase::Defeat => {
                let lines = [
                    ("HP restaurés à 50%", RED_ERR),
                    ("[ Espace ]  pour continuer", CYAN),
                ];
                let line_h = menu_h / (lines.len() as f32 + 1.0);
                for (i, (txt, col)) in lines.iter().enumerate() {
                    let tw = measure_text(txt, None, 26, 1.0).width;
                    draw_text(txt, phys_w * 0.5 - tw * 0.5, MENU_Y + line_h * (i as f32 + 1.2), 26.0, *col);
                }
            }
        }

        // ── Overlay inventaire ────────────────────────────────────────────
        if let TurnPhase::ItemMenu { selected } = self.turn {
            inventory.render_menu(phys_w, phys_h, selected, items);
        }
    }

    // ── Helpers de rendu ─────────────────────────────────────────────────

    fn draw_log_lines(&self, n: usize) {
        let last: Vec<&str> = self.log.iter().rev().take(n)
            .map(|s| s.as_str()).collect::<Vec<_>>().into_iter().rev().collect();
        for (i, line) in last.iter().enumerate() {
            draw_text(line, 16.0, SCENE_H + 24.0 + i as f32 * 26.0, 20.0, WHITE);
        }
    }

    fn draw_move_grid(&self, phys_w: f32, menu_h: f32, attacks: &[AttackData]) {
        let col_w = phys_w * 0.5;
        let row_h = menu_h * 0.5;
        let moves = &self.player_creature.moves;
        let bag_slot = moves.len(); // Bag occupe le slot juste après le dernier move

        for i in 0..4 {
            let col = (i % 2) as f32;
            let row = (i / 2) as f32;
            let cx  = col * col_w;
            let cy  = MENU_Y + row * row_h;

            let is_bag = moves.len() < 4 && i == bag_slot;
            let is_move = i < moves.len();

            if is_move {
                let selected = i == self.selected_move;
                if selected {
                    draw_rectangle(cx + 2.0, cy + 2.0, col_w - 4.0, row_h - 4.0, SEL_HL);
                    draw_rectangle_lines(cx + 2.0, cy + 2.0, col_w - 4.0, row_h - 4.0, 2.0, CYAN);
                }
                let move_id = &moves[i];
                let atk = attacks.iter().find(|a| a.id == *move_id);
                let move_name = atk.map(|a| a.name.as_str()).unwrap_or(move_id.as_str());
                let txt_col = if selected { CYAN } else { WHITE };
                let type_label = atk.map(|a| a.attack_type.as_str()).unwrap_or("");
                draw_text(move_name, cx + 20.0, cy + row_h * 0.44, 26.0, txt_col);
                draw_text(type_label, cx + 20.0, cy + row_h * 0.76, 16.0, PP_COL);
            } else if is_bag {
                let selected = i == self.selected_move;
                if selected {
                    draw_rectangle(cx + 2.0, cy + 2.0, col_w - 4.0, row_h - 4.0, SEL_HL);
                    draw_rectangle_lines(cx + 2.0, cy + 2.0, col_w - 4.0, row_h - 4.0, 2.0, CYAN);
                }
                let txt_col = if selected { CYAN } else { WHITE };
                draw_text("Bag", cx + 20.0, cy + row_h * 0.44, 26.0, txt_col);
                draw_text("[inventaire]", cx + 20.0, cy + row_h * 0.74, 16.0, PP_COL);
            } else {
                draw_text("---", cx + 20.0, cy + row_h * 0.44, 26.0, DIM);
            }

            if i % 2 == 0 && i + 1 < 4 {
                draw_line(col_w, cy, col_w, cy + row_h, 1.0, CYAN);
            }
            if i < 2 {
                draw_line(cx, MENU_Y + row_h, cx + col_w, MENU_Y + row_h, 1.0, CYAN);
            }
        }
    }
}

// ── Helpers module-privés ─────────────────────────────────────────────────

/// Formule canonique : `(atk * power) / (def_eff * 2) + 2`, minimum 1.
/// Si `def_boost` est actif, la défense effective est multipliée par 1.3.
pub fn calculate_damage(atk: u32, power: u32, def: u32, def_boost: bool) -> u32 {
    let def_mult: u32 = if def_boost { 130 } else { 100 };
    let eff_defense = ((def * def_mult) / 100).max(1);
    ((atk * power) / (eff_defense * 2) + 2).max(1)
}

fn type_color(t: &str) -> Color {
    match t {
        "prompt"  => Color { r: 0.18, g: 0.32, b: 0.82, a: 1.0 },
        "brute"   => Color { r: 0.72, g: 0.10, b: 0.10, a: 1.0 },
        "exploit" => Color { r: 0.08, g: 0.62, b: 0.18, a: 1.0 },
        _         => Color { r: 0.32, g: 0.32, b: 0.32, a: 1.0 },
    }
}

fn draw_hp_bar(x: f32, y: f32, w: f32, h: f32, current: u32, max: u32) {
    let ratio = if max == 0 { 0.0_f32 } else { current as f32 / max as f32 };
    let bar_col = if ratio > 0.5 {
        Color { r: 0.0,  g: 0.88, b: 0.78, a: 1.0 }
    } else if ratio > 0.25 {
        Color { r: 0.88, g: 0.78, b: 0.0,  a: 1.0 }
    } else {
        Color { r: 0.88, g: 0.18, b: 0.08, a: 1.0 }
    };
    draw_rectangle(x, y, w, h, Color { r: 0.12, g: 0.12, b: 0.12, a: 1.0 });
    if ratio > 0.0 {
        draw_rectangle(x, y, w * ratio, h, bar_col);
    }
    draw_rectangle_lines(x, y, w, h, 1.5, CYAN);
}

#[cfg(test)]
mod tests {
    use super::calculate_damage;

    #[test]
    fn no_boost_basic_damage() {
        // (10*25) / (9*2) + 2 = 250/18 + 2 = 13 + 2 = 15
        assert_eq!(calculate_damage(10, 25, 9, false), 15);
    }

    #[test]
    fn def_boost_reduces_damage() {
        // eff_def = (9*130/100).max(1) = 11
        // (10*25) / (11*2) + 2 = 250/22 + 2 = 11 + 2 = 13
        assert_eq!(calculate_damage(10, 25, 9, true), 13);
    }

    #[test]
    fn zero_power_returns_minimum() {
        assert_eq!(calculate_damage(10, 0, 9, false), 2);
    }

    #[test]
    fn zero_defense_does_not_panic() {
        // eff_defense = (0 * 100 / 100).max(1) = 1
        let dmg = calculate_damage(10, 25, 0, false);
        assert!(dmg >= 1);
    }
}

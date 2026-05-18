use macroquad::prelude::*;

use crate::data::NpcData;

const TILE_SIZE: f32 = 16.0;
const CYAN: Color = Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };

pub enum DialogueResult {
    Continue,
    Done,
    TriggerBattle,
}

pub struct NpcInstance {
    pub data: NpcData,
    texture: Texture2D,
    pub dialogue_index: usize,
    pub is_talking: bool,
    pub defeated: bool,
}

impl NpcInstance {
    /// Crée un NPC en réutilisant une texture déjà chargée (clone léger par refcount).
    pub fn new(data: NpcData, texture: Texture2D) -> NpcInstance {
        NpcInstance { data, texture, dialogue_index: 0, is_talking: false, defeated: false }
    }

    pub fn render(&self) {
        draw_texture_ex(
            &self.texture,
            self.data.tile_x as f32 * TILE_SIZE,
            self.data.tile_y as f32 * TILE_SIZE,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(TILE_SIZE, TILE_SIZE)),
                ..Default::default()
            },
        );
    }

    pub fn start_dialogue(&mut self) {
        self.is_talking = true;
        self.dialogue_index = 0;
    }

    pub fn advance_dialogue(&mut self) -> DialogueResult {
        let lines = self.current_dialogue_lines();
        if self.dialogue_index + 1 >= lines.len() {
            self.is_talking = false;
            if self.data.is_trainer && !self.defeated {
                return DialogueResult::TriggerBattle;
            }
            return DialogueResult::Done;
        }
        self.dialogue_index += 1;
        DialogueResult::Continue
    }

    fn current_dialogue_lines(&self) -> &[String] {
        if self.defeated
            && let Some(dl) = &self.data.dialogue_defeated {
                return dl.as_slice();
            }
        &self.data.dialogue
    }

    pub fn render_dialogue_box(&self, phys_w: f32, phys_h: f32) {
        let lines = self.current_dialogue_lines();
        let text = lines
            .get(self.dialogue_index)
            .map(|s| s.as_str())
            .unwrap_or("");

        let margin = 16.0;
        let box_h = 120.0;
        let box_y = phys_h - box_h - margin;
        let box_w = phys_w - margin * 2.0;
        let box_x = margin;

        draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.02, g: 0.04, b: 0.10, a: 0.97 });
        draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

        draw_text(&self.data.name, box_x + 12.0, box_y + 28.0, 24.0, CYAN);
        draw_line(box_x + 12.0, box_y + 36.0, box_x + box_w - 12.0, box_y + 36.0, 1.0, CYAN);

        let wrapped = wrap_text(text, 52);
        for (i, line) in wrapped.iter().take(3).enumerate() {
            draw_text(line, box_x + 12.0, box_y + 60.0 + i as f32 * 22.0, 20.0, WHITE);
        }

        if (get_time() % 0.8) < 0.5 {
            let hint = "[ Espace ]";
            let hw = measure_text(hint, None, 16, 1.0).width;
            draw_text(hint, box_x + box_w - hw - 10.0, box_y + box_h - 8.0, 16.0, CYAN);
        }
    }
}

/// Charge la texture NPC une seule fois ; retourne un fallback magenta si fichier absent.
pub fn load_npc_texture() -> Texture2D {
    let texture = match std::fs::read("assets/sprites/npc_generic.png") {
        Ok(bytes) => Texture2D::from_file_with_format(&bytes, Some(ImageFormat::Png)),
        Err(e) => {
            eprintln!("sprite NPC introuvable : {e}");
            Texture2D::from_image(&Image::gen_image_color(16, 16, MAGENTA))
        }
    };
    texture.set_filter(FilterMode::Nearest);
    texture
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
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

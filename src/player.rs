use macroquad::prelude::*;
use ::rand::Rng;

use crate::world::GameMap;

const TILE_SIZE: f32 = 16.0;
const MOVE_DURATION: f32 = 0.15;

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Down,
    Left,
    Right,
    Up,
}

impl Direction {
    fn frame_row(self) -> u32 {
        match self {
            Direction::Down => 0,
            Direction::Left => 1,
            Direction::Right => 2,
            Direction::Up => 3,
        }
    }
}

pub struct Player {
    pub tile_x: i32,
    pub tile_y: i32,
    pub pixel_x: f32,
    pub pixel_y: f32,
    pub direction: Direction,
    pub is_moving: bool,
    pub trigger_encounter: bool,
    move_timer: f32,
    target_px: f32,
    target_py: f32,
    steps_in_grass: u8,
    texture: Texture2D,
}

impl Player {
    pub fn new(start_tile_x: i32, start_tile_y: i32) -> Player {
        let texture = match std::fs::read("assets/sprites/player.png") {
            Ok(bytes) => Texture2D::from_file_with_format(&bytes, Some(ImageFormat::Png)),
            Err(e) => {
                eprintln!("sprite joueur introuvable : {e}");
                Texture2D::from_image(&Image::gen_image_color(16, 64, MAGENTA))
            }
        };
        texture.set_filter(FilterMode::Nearest);

        let px = start_tile_x as f32 * TILE_SIZE;
        let py = start_tile_y as f32 * TILE_SIZE;
        Player {
            tile_x: start_tile_x,
            tile_y: start_tile_y,
            pixel_x: px,
            pixel_y: py,
            direction: Direction::Down,
            is_moving: false,
            trigger_encounter: false,
            move_timer: 0.0,
            target_px: px,
            target_py: py,
            steps_in_grass: 0,
            texture,
        }
    }

    // Retourne true quand un pas complet vient d'être effectué.
    pub fn update(&mut self, map: &GameMap, npc_tiles: &[(i32, i32)], dt: f32) -> bool {
        if self.is_moving {
            self.move_timer += dt;
            let t = (self.move_timer / MOVE_DURATION).min(1.0);
            self.pixel_x = self.pixel_x + (self.target_px - self.pixel_x) * t;
            self.pixel_y = self.pixel_y + (self.target_py - self.pixel_y) * t;

            if self.move_timer >= MOVE_DURATION {
                self.pixel_x = self.target_px;
                self.pixel_y = self.target_py;
                self.is_moving = false;
                self.move_timer = 0.0;
                if map.is_encounter_tile(self.tile_x, self.tile_y) {
                    self.steps_in_grass += 1;
                    let threshold: u8 = ::rand::thread_rng().gen_range(8..17);
                    if self.steps_in_grass >= threshold {
                        self.trigger_encounter = true;
                        self.steps_in_grass = 0;
                    }
                } else {
                    self.steps_in_grass = 0;
                }
                return true;
            }
            return false;
        }

        let (mut dx, mut dy) = (0i32, 0i32);
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            dy = -1;
            self.direction = Direction::Up;
        } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            dy = 1;
            self.direction = Direction::Down;
        } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            dx = -1;
            self.direction = Direction::Left;
        } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            dx = 1;
            self.direction = Direction::Right;
        }

        if dx != 0 || dy != 0 {
            let nx = self.tile_x + dx;
            let ny = self.tile_y + dy;
            if map.is_walkable(nx, ny) && !npc_tiles.contains(&(nx, ny)) {
                self.tile_x = nx;
                self.tile_y = ny;
                self.target_px = nx as f32 * TILE_SIZE;
                self.target_py = ny as f32 * TILE_SIZE;
                self.is_moving = true;
            }
        }

        false
    }

    pub fn teleport(&mut self, tile_x: i32, tile_y: i32) {
        self.tile_x = tile_x;
        self.tile_y = tile_y;
        self.pixel_x = tile_x as f32 * TILE_SIZE;
        self.pixel_y = tile_y as f32 * TILE_SIZE;
        self.target_px = self.pixel_x;
        self.target_py = self.pixel_y;
        self.is_moving = false;
        self.move_timer = 0.0;
    }

    pub fn render(&self) {
        let row = self.direction.frame_row();
        draw_texture_ex(
            &self.texture,
            self.pixel_x,
            self.pixel_y,
            WHITE,
            DrawTextureParams {
                source: Some(Rect::new(0.0, row as f32 * TILE_SIZE, TILE_SIZE, TILE_SIZE)),
                dest_size: Some(vec2(TILE_SIZE, TILE_SIZE)),
                ..Default::default()
            },
        );
    }
}

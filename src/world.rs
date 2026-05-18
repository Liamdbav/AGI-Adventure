use std::collections::HashSet;

use macroquad::prelude::*;
use tiled::{Loader, LayerType, PropertyValue, TileLayer};

pub struct GameMap {
    pub width: u32,
    pub height: u32,
    pub tile_size: f32,
    tileset: Texture2D,
    tileset_cols: u32,
    layers: Vec<Vec<Option<u32>>>,
    canopy: Vec<Option<u32>>,
    collision: Vec<Vec<bool>>,
    encounter_tiles: Vec<Vec<bool>>,
}

impl GameMap {
    pub fn load(path: &str) -> Result<GameMap, String> {
        let mut loader = Loader::new();
        let map = loader.load_tmx_map(path)
            .map_err(|e| format!("impossible de charger la map TMX '{}' : {}", path, e))?;

        let tile_size = map.tile_width as f32;
        let width = map.width;
        let height = map.height;

        let tileset_arc = map.tilesets().first()
            .ok_or_else(|| format!("aucun tileset dans la map '{}'", path))?
            .clone();
        let image = tileset_arc.image.as_ref()
            .ok_or_else(|| format!("tileset sans image source dans '{}'", path))?;
        let bytes = std::fs::read(&image.source)
            .map_err(|e| format!("impossible de lire l'image du tileset '{}' : {}", image.source.display(), e))?;
        let tileset = Texture2D::from_file_with_format(&bytes, Some(ImageFormat::Png));
        tileset.set_filter(FilterMode::Nearest);
        let tileset_cols = tileset_arc.columns;

        let mut collidable_ids: HashSet<u32> = HashSet::new();
        let mut encounter_ids: HashSet<u32> = HashSet::new();
        for (id, tile) in tileset_arc.tiles() {
            if let Some(PropertyValue::BoolValue(true)) = tile.properties.get("collidable") {
                collidable_ids.insert(id);
            }
            if let Some(PropertyValue::BoolValue(true)) = tile.properties.get("encounter_zone") {
                encounter_ids.insert(id);
            }
        }

        let mut ground: Vec<Option<u32>> = vec![None; (width * height) as usize];
        let mut obstacles: Vec<Option<u32>> = vec![None; (width * height) as usize];
        let mut canopy: Vec<Option<u32>> = vec![None; (width * height) as usize];
        let mut collision: Vec<Vec<bool>> = vec![vec![false; width as usize]; height as usize];
        let mut encounter_tiles: Vec<Vec<bool>> = vec![vec![false; width as usize]; height as usize];

        for layer in map.layers() {
            let LayerType::Tiles(tl) = layer.layer_type() else {
                continue;
            };
            let TileLayer::Finite(data) = tl else {
                continue;
            };
            let target: &mut Vec<Option<u32>> = match layer.name.as_str() {
                "Ground" => &mut ground,
                "Collision" => &mut obstacles,
                "Canopy" => &mut canopy,
                _ => continue,
            };
            for y in 0..height {
                for x in 0..width {
                    if let Some(t) = data.get_tile(x as i32, y as i32) {
                        let local_id = t.id();
                        target[(y * width + x) as usize] = Some(local_id);
                        if layer.name == "Collision" || collidable_ids.contains(&local_id) {
                            collision[y as usize][x as usize] = true;
                        }
                        if layer.name == "Ground" && encounter_ids.contains(&local_id) {
                            encounter_tiles[y as usize][x as usize] = true;
                        }
                    }
                }
            }
        }

        Ok(GameMap {
            width,
            height,
            tile_size,
            tileset,
            tileset_cols,
            layers: vec![ground, obstacles],
            canopy,
            collision,
            encounter_tiles,
        })
    }

    pub fn render(&self, camera_offset: Vec2) {
        for layer in &self.layers {
            for y in 0..self.height {
                for x in 0..self.width {
                    let Some(id) = layer[(y * self.width + x) as usize] else {
                        continue;
                    };
                    let sx = (id % self.tileset_cols) as f32 * self.tile_size;
                    let sy = (id / self.tileset_cols) as f32 * self.tile_size;
                    let dx = x as f32 * self.tile_size - camera_offset.x;
                    let dy = y as f32 * self.tile_size - camera_offset.y;
                    draw_texture_ex(
                        &self.tileset,
                        dx,
                        dy,
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new(sx, sy, self.tile_size, self.tile_size)),
                            dest_size: Some(vec2(self.tile_size, self.tile_size)),
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    pub fn render_canopy(&self, camera_offset: Vec2) {
        for y in 0..self.height {
            for x in 0..self.width {
                let Some(id) = self.canopy[(y * self.width + x) as usize] else {
                    continue;
                };
                let sx = (id % self.tileset_cols) as f32 * self.tile_size;
                let sy = (id / self.tileset_cols) as f32 * self.tile_size;
                let dx = x as f32 * self.tile_size - camera_offset.x;
                let dy = y as f32 * self.tile_size - camera_offset.y;
                draw_texture_ex(
                    &self.tileset,
                    dx,
                    dy,
                    WHITE,
                    DrawTextureParams {
                        source: Some(Rect::new(sx, sy, self.tile_size, self.tile_size)),
                        dest_size: Some(vec2(self.tile_size, self.tile_size)),
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub fn is_walkable(&self, tile_x: i32, tile_y: i32) -> bool {
        if tile_x < 0
            || tile_y < 0
            || tile_x >= self.width as i32
            || tile_y >= self.height as i32
        {
            return false;
        }
        !self.collision[tile_y as usize][tile_x as usize]
    }

    pub fn is_encounter_tile(&self, tile_x: i32, tile_y: i32) -> bool {
        if tile_x < 0
            || tile_y < 0
            || tile_x >= self.width as i32
            || tile_y >= self.height as i32
        {
            return false;
        }
        self.encounter_tiles[tile_y as usize][tile_x as usize]
    }

    pub fn render_minimap(&self, x: f32, y: f32, w: f32, h: f32, player_tx: i32, player_ty: i32) {
        let tw = w / self.width as f32;
        let th = h / self.height as f32;
        for ty in 0..self.height {
            for tx in 0..self.width {
                let px = x + tx as f32 * tw;
                let py = y + ty as f32 * th;
                let col = if self.collision[ty as usize][tx as usize] {
                    Color { r: 0.06, g: 0.06, b: 0.10, a: 1.0 }
                } else if self.encounter_tiles[ty as usize][tx as usize] {
                    Color { r: 0.08, g: 0.28, b: 0.12, a: 1.0 }
                } else {
                    Color { r: 0.18, g: 0.20, b: 0.26, a: 1.0 }
                };
                draw_rectangle(px, py, tw.max(1.0), th.max(1.0), col);
            }
        }
        if (get_time() % 0.9) < 0.55 {
            let px = x + player_tx as f32 * tw + tw * 0.5;
            let py = y + player_ty as f32 * th + th * 0.5;
            draw_circle(px, py, (tw.min(th) * 0.5).max(2.5),
                Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 });
        }
    }
}

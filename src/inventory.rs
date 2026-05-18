use std::collections::HashMap;

use macroquad::prelude::*;

use crate::data::ItemData;

const CYAN: Color = Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
const DIM:  Color = Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 };

// Ordre d'affichage fixe des objets
const ITEM_ORDER: &[&str] = &["patch", "capture_flag"];

pub struct Inventory {
    pub items: HashMap<String, u32>,
}

impl Inventory {
    pub fn new() -> Inventory {
        let mut items = HashMap::new();
        items.insert("patch".to_string(), 3);
        items.insert("capture_flag".to_string(), 5);
        Inventory { items }
    }

    pub fn from_items(items: HashMap<String, u32>) -> Inventory {
        Inventory { items }
    }

    /// Consomme un exemplaire de l'objet. Retourne false si épuisé.
    pub fn use_item(&mut self, id: &str) -> bool {
        if let Some(count) = self.items.get_mut(id)
            && *count > 0 {
                *count -= 1;
                return true;
            }
        false
    }

    /// Liste ordonnée des (id, count) présents dans l'inventaire.
    pub fn ordered_ids(&self) -> Vec<&str> {
        ITEM_ORDER.iter()
            .filter(|id| self.items.get(**id).copied().unwrap_or(0) > 0).copied()
            .collect()
    }

    /// Affiche l'inventaire en overlay centré (coordonnées physiques).
    pub fn render_menu(&self, phys_w: f32, phys_h: f32, selected: usize, items_data: &[ItemData]) {
        // Fond semi-opaque couvrant tout l'écran
        draw_rectangle(0.0, 0.0, phys_w, phys_h, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.88 });

        let box_w   = 420.0;
        let row_h   = 46.0;
        let items   = self.ordered_ids();
        let rows    = items.len().max(1) as f32;
        let box_h   = 64.0 + rows * row_h + 32.0;
        let box_x   = phys_w * 0.5 - box_w * 0.5;
        let box_y   = phys_h * 0.5 - box_h * 0.5;

        draw_rectangle(box_x, box_y, box_w, box_h, Color { r: 0.02, g: 0.04, b: 0.10, a: 1.0 });
        draw_rectangle_lines(box_x, box_y, box_w, box_h, 2.0, CYAN);

        let title = "INVENTAIRE";
        let tw = measure_text(title, None, 28, 1.0).width;
        draw_text(title, phys_w * 0.5 - tw * 0.5, box_y + 36.0, 28.0, CYAN);
        draw_line(box_x + 12.0, box_y + 46.0, box_x + box_w - 12.0, box_y + 46.0, 1.0, CYAN);

        if items.is_empty() {
            let msg = "Inventaire vide.";
            let mw = measure_text(msg, None, 22, 1.0).width;
            draw_text(msg, phys_w * 0.5 - mw * 0.5, box_y + 64.0 + row_h * 0.5, 22.0, DIM);
        } else {
            for (i, id) in items.iter().enumerate() {
                let count = self.items.get(*id).copied().unwrap_or(0);
                let name  = items_data.iter()
                    .find(|d| d.id == *id)
                    .map(|d| d.name.as_str())
                    .unwrap_or(id);

                let cy = box_y + 60.0 + i as f32 * row_h;
                let selected_row = i == selected;

                if selected_row {
                    draw_rectangle(box_x + 4.0, cy, box_w - 8.0, row_h - 4.0,
                        Color { r: 0.0, g: 0.22, b: 0.22, a: 1.0 });
                    draw_rectangle_lines(box_x + 4.0, cy, box_w - 8.0, row_h - 4.0, 1.5, CYAN);
                }

                let col = if selected_row { CYAN } else { WHITE };
                draw_text(name, box_x + 20.0, cy + row_h * 0.62, 24.0, col);

                let qty = format!("x{}", count);
                let qw  = measure_text(&qty, None, 24, 1.0).width;
                draw_text(&qty, box_x + box_w - qw - 20.0, cy + row_h * 0.62, 24.0, col);
            }
        }

        let hint = "[ Espace ] utiliser  |  [ Échap ] fermer";
        let hw = measure_text(hint, None, 16, 1.0).width;
        draw_text(hint, phys_w * 0.5 - hw * 0.5, box_y + box_h - 10.0, 16.0, DIM);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_item_decrements_count() {
        let mut inv = Inventory::new();
        assert!(inv.use_item("patch"));
        assert_eq!(*inv.items.get("patch").unwrap(), 2);
    }

    #[test]
    fn use_item_returns_false_when_empty() {
        let mut inv = Inventory::from_items(HashMap::new());
        assert!(!inv.use_item("patch"));
    }

    #[test]
    fn use_item_exhausts_to_zero() {
        let mut items = HashMap::new();
        items.insert("patch".to_string(), 1);
        let mut inv = Inventory::from_items(items);
        assert!(inv.use_item("patch"));
        assert!(!inv.use_item("patch"));
    }

    #[test]
    fn ordered_ids_excludes_zero_count() {
        let mut items = HashMap::new();
        items.insert("patch".to_string(), 0);
        items.insert("capture_flag".to_string(), 2);
        let inv = Inventory::from_items(items);
        let ids = inv.ordered_ids();
        assert!(!ids.contains(&"patch"));
        assert!(ids.contains(&"capture_flag"));
    }

    #[test]
    fn ordered_ids_respects_item_order() {
        let inv = Inventory::new();
        let ids = inv.ordered_ids();
        assert_eq!(ids, vec!["patch", "capture_flag"]);
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::creature::CreatureInstance;

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub player_name: String,
    pub player_tile_x: i32,
    pub player_tile_y: i32,
    pub current_map: String,
    pub player_creature: CreatureInstance,
    pub inventory: HashMap<String, u32>,
    pub defeated_trainers: Vec<String>,
}

impl SaveData {
    pub fn save(&self, path: &str) {
        let json = serde_json::to_string_pretty(self).expect("sérialisation save échouée");
        let tmp = format!("{}.tmp", path);
        std::fs::write(&tmp, &json).expect("écriture save.json.tmp échouée");
        std::fs::rename(&tmp, path).expect("renommage save.json échoué");
    }

    pub fn load(path: &str) -> Option<SaveData> {
        let content = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str(&content) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("save.json corrompu, ignoré : {e}");
                None
            }
        }
    }

    pub fn from_game_state(
        player_name: &str,
        player_tile_x: i32,
        player_tile_y: i32,
        current_map: &str,
        player_creature: &CreatureInstance,
        inventory: &HashMap<String, u32>,
        defeated_trainers: &[String],
    ) -> SaveData {
        SaveData {
            player_name: player_name.to_string(),
            player_tile_x,
            player_tile_y,
            current_map: current_map.to_string(),
            player_creature: player_creature.clone(),
            inventory: inventory.clone(),
            defeated_trainers: defeated_trainers.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_creature() -> CreatureInstance {
        CreatureInstance {
            data_id: "bob".to_string(),
            name: "Bob".to_string(),
            level: 5,
            creature_type: vec!["prompt".to_string()],
            current_hp: 19,
            max_hp: 19,
            attack: 9,
            defense: 9,
            speed: 9,
            moves: vec!["surcharge".to_string()],
            catch_rate: 255,
            defense_boost_turns: 0,
            speed_down_turns: 0,
        }
    }

    #[test]
    fn round_trip_serialization() {
        let mut inv = HashMap::new();
        inv.insert("patch".to_string(), 3u32);
        let original = SaveData {
            player_name: "Liam".to_string(),
            player_tile_x: 5,
            player_tile_y: 4,
            current_map: "town".to_string(),
            player_creature: dummy_creature(),
            inventory: inv,
            defeated_trainers: vec!["script_kiddies".to_string()],
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let restored: SaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.player_name, "Liam");
        assert_eq!(restored.player_tile_x, 5);
        assert_eq!(restored.player_creature.level, 5);
        assert_eq!(restored.inventory.get("patch"), Some(&3));
        assert_eq!(restored.defeated_trainers, vec!["script_kiddies"]);
    }

    #[test]
    fn load_returns_none_on_corrupt_json() {
        let result = SaveData::load("/tmp/agi_test_nonexistent_corrupt.json");
        assert!(result.is_none());
    }
}

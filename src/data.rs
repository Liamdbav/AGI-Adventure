use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AttackData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub power: u32,
    pub accuracy: u32,
    pub pp: u32,
    #[serde(rename = "type")]
    pub attack_type: String,
    pub category: String,
    pub effect: Option<String>,
    pub effect_value: Option<i32>,
    pub effect_duration: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreatureData {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub creature_type: Vec<String>,
    pub description: String,
    pub base_hp: u32,
    pub base_attack: u32,
    pub base_defense: u32,
    pub base_speed: u32,
    pub catch_rate: u32,
    pub learnable_moves: Vec<String>,
    pub wild_levels: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ItemData {
    pub id: String,
    pub name: String,
    pub description: String,
    pub effect: String,
    pub value: i32,
    pub price: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrainerCreature {
    pub creature_id: String,
    pub level: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NpcData {
    pub id: String,
    pub name: String,
    pub tile_x: i32,
    pub tile_y: i32,
    pub map: String,
    pub is_trainer: bool,
    pub dialogue: Vec<String>,
    pub dialogue_defeated: Option<Vec<String>>,
    pub trainer_team: Option<Vec<TrainerCreature>>,
}

pub struct GameData {
    pub attacks: Vec<AttackData>,
    pub creatures: Vec<CreatureData>,
    pub items: Vec<ItemData>,
    pub npcs: Vec<NpcData>,
}

impl GameData {
    pub fn load() -> Result<GameData, String> {
        let attacks_raw = std::fs::read_to_string("assets/data/attacks.json")
            .map_err(|e| format!("assets/data/attacks.json introuvable : {e}"))?;
        let creatures_raw = std::fs::read_to_string("assets/data/creatures.json")
            .map_err(|e| format!("assets/data/creatures.json introuvable : {e}"))?;
        let items_raw = std::fs::read_to_string("assets/data/items.json")
            .map_err(|e| format!("assets/data/items.json introuvable : {e}"))?;
        let npcs_raw = std::fs::read_to_string("assets/data/npcs.json")
            .map_err(|e| format!("assets/data/npcs.json introuvable : {e}"))?;

        let attacks: Vec<AttackData> = serde_json::from_str(&attacks_raw)
            .map_err(|e| format!("erreur de parsing attacks.json : {e}"))?;
        let creatures: Vec<CreatureData> = serde_json::from_str(&creatures_raw)
            .map_err(|e| format!("erreur de parsing creatures.json : {e}"))?;
        let items: Vec<ItemData> = serde_json::from_str(&items_raw)
            .map_err(|e| format!("erreur de parsing items.json : {e}"))?;
        let npcs: Vec<NpcData> = serde_json::from_str(&npcs_raw)
            .map_err(|e| format!("erreur de parsing npcs.json : {e}"))?;

        Ok(GameData { attacks, creatures, items, npcs })
    }

    pub fn get_creature(&self, id: &str) -> &CreatureData {
        self.creatures.iter().find(|c| c.id == id)
            .unwrap_or_else(|| panic!("créature inconnue : {id}"))
    }

    #[allow(dead_code)]
    pub fn get_attack(&self, id: &str) -> &AttackData {
        self.attacks.iter().find(|a| a.id == id)
            .unwrap_or_else(|| panic!("attaque inconnue : {id}"))
    }

    pub fn get_npcs_for_map(&self, map_id: &str) -> Vec<&NpcData> {
        self.npcs.iter().filter(|n| n.map == map_id).collect()
    }
}

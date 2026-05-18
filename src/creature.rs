use serde::{Deserialize, Serialize};

use crate::data::CreatureData;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreatureInstance {
    pub data_id: String,
    pub name: String,
    pub level: u8,
    pub creature_type: Vec<String>,
    pub current_hp: u32,
    pub max_hp: u32,
    pub attack: u32,
    pub defense: u32,
    pub speed: u32,
    pub moves: Vec<String>,
    pub catch_rate: u32,
    pub defense_boost_turns: u8,
    pub speed_down_turns: u8,
}

impl CreatureInstance {
    pub fn from_data(data: &CreatureData, level: u8) -> CreatureInstance {
        let lv = level as u32;
        let max_hp = (data.base_hp * lv) / 50 + lv + 10;
        let attack = (data.base_attack * lv) / 50 + 5;
        let defense = (data.base_defense * lv) / 50 + 5;
        let speed = (data.base_speed * lv) / 50 + 5;
        let moves = data.learnable_moves.iter().take(4).cloned().collect();

        CreatureInstance {
            data_id: data.id.clone(),
            name: data.name.clone(),
            level,
            creature_type: data.creature_type.clone(),
            current_hp: max_hp,
            max_hp,
            attack,
            defense,
            speed,
            moves,
            catch_rate: data.catch_rate,
            defense_boost_turns: 0,
            speed_down_turns: 0,
        }
    }

    #[allow(dead_code)]
    pub fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CreatureData;

    fn bob_data() -> CreatureData {
        CreatureData {
            id: "bob".to_string(),
            name: "Bob".to_string(),
            creature_type: vec!["prompt".to_string()],
            description: String::new(),
            base_hp: 45,
            base_attack: 49,
            base_defense: 49,
            base_speed: 45,
            catch_rate: 255,
            learnable_moves: vec!["surcharge".to_string(), "guardrail".to_string()],
            wild_levels: vec![2, 7],
        }
    }

    #[test]
    fn bob_lv5_stats() {
        let bob = CreatureInstance::from_data(&bob_data(), 5);
        assert_eq!(bob.max_hp, 19);
        assert_eq!(bob.current_hp, 19);
        assert_eq!(bob.attack, 9);
        assert_eq!(bob.defense, 9);
        assert_eq!(bob.speed, 9);
    }

    #[test]
    fn from_data_sets_catch_rate_and_type() {
        let bob = CreatureInstance::from_data(&bob_data(), 1);
        assert_eq!(bob.catch_rate, 255);
        assert_eq!(bob.creature_type, vec!["prompt"]);
    }

    #[test]
    fn from_data_takes_up_to_four_moves() {
        let mut data = bob_data();
        data.learnable_moves = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string(), "e".to_string()];
        let c = CreatureInstance::from_data(&data, 1);
        assert_eq!(c.moves.len(), 4);
    }
}

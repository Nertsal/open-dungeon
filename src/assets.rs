use crate::prelude::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    pub controls: Controls,
    pub config: Config,
}

#[derive(geng::asset::Load, Debug, Clone, Serialize, Deserialize)]
#[load(serde = "ron")]
pub struct Controls {
    pub left: Vec<geng_utils::key::EventKey>,
    pub right: Vec<geng_utils::key::EventKey>,
    pub down: Vec<geng_utils::key::EventKey>,
    pub up: Vec<geng_utils::key::EventKey>,

    pub draw: Vec<geng_utils::key::EventKey>,
}

#[derive(geng::asset::Load, Debug, Clone, Serialize, Deserialize)]
#[load(serde = "ron")]
pub struct Config {
    pub player: PlayerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub health: Hp,
    pub speed: Coord,
    pub acceleration: Coord,
    pub dash: DashConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyConfig {
    pub health: Hp,
    pub speed: Coord,
    pub acceleration: Coord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashConfig {
    pub max_distance: Coord,
    pub speed: Coord,
    pub width: Coord,
    pub damage: Hp,
}

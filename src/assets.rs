use crate::prelude::Coord;

use geng::prelude::*;

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

#[derive(geng::asset::Load, Debug, Clone, Serialize, Deserialize)]
#[load(serde = "ron")]
pub struct PlayerConfig {
    pub speed: Coord,
}

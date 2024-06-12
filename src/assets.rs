use crate::prelude::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    pub controls: Controls,
    pub config: Config,
    pub palette: Palette,
    pub sounds: Sounds,
}

#[derive(geng::asset::Load)]
pub struct Sounds {
    #[load(options(looped = "true"))]
    pub drawing: geng::Sound,
    pub hit: geng::Sound,
    pub kill: geng::Sound,
    pub hit_self: geng::Sound,
    pub bounce: geng::Sound,
    pub expand: geng::Sound,
}

#[derive(geng::asset::Load, Debug, Clone, Serialize, Deserialize)]
#[load(serde = "ron")]
pub struct Palette {
    pub background: Rgba<f32>,
    pub room: Rgba<f32>,
    pub wall: Rgba<f32>,
    pub wall_block: Rgba<f32>,
    pub player: Rgba<f32>,
    pub object: Rgba<f32>,
    pub enemy: Rgba<f32>,
    pub health: Rgba<f32>,
    pub drawing: Rgba<f32>,
    pub dash: Rgba<f32>,
    pub damage: Rgba<f32>,
    pub collision: Rgba<f32>,
    pub upgrade: Rgba<f32>,
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
    pub starting_area: vec2<Coord>,
    pub player: PlayerConfig,
    pub enemies: Vec<EnemyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub health: Hp,
    pub speed: Coord,
    pub acceleration: Coord,
    pub dash: DashConfig,
    pub shape: Shape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyConfig {
    pub cost: Option<R32>,
    pub health: Hp,
    pub speed: Coord,
    pub acceleration: Coord,
    pub shape: Shape,
    pub ai: EnemyAI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashConfig {
    pub max_distance: Coord,
    pub speed: Coord,
    pub width: Coord,
    pub damage: Hp,
}

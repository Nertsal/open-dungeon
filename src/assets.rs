use crate::prelude::*;

#[derive(geng::asset::Load)]
pub struct Assets {
    pub controls: Controls,
    pub config: Config,
    pub palette: Palette,
    pub sounds: Sounds,
    pub sprites: Sprites,
    pub shaders: Shaders,
    #[load(path = "font/font.ttf")]
    pub font: Rc<geng::Font>,
}

#[derive(geng::asset::Load)]
pub struct Shaders {
    pub vhs: ugli::Program,
    pub background: ugli::Program,
}

#[derive(geng::asset::Load)]
pub struct Sounds {
    #[load(options(looped = "true"))]
    pub drawing: geng::Sound,
    #[load(options(looped = "true"))]
    pub helicopter: geng::Sound,
    pub hit: geng::Sound,
    pub kill: geng::Sound,
    pub hit_self: geng::Sound,
    pub bounce: geng::Sound,
    pub expand: geng::Sound,
    pub minigun: geng::Sound,
    pub explosion: geng::Sound,
}

#[derive(geng::asset::Load)]
pub struct Sprites {
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub width: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub range: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub damage: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub speed: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub heal: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub skull: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub whip: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub dash: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub bow: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub fishing_rod: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub easy: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub medium: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub hard: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub hint: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub barrel: ugli::Texture,
    #[load(options(filter = "ugli::Filter::Nearest"))]
    pub producer: ugli::Texture,
}

#[derive(geng::asset::Load, Debug, Clone, Serialize, Deserialize)]
#[load(serde = "ron")]
pub struct Palette {
    pub background: Rgba<f32>,
    pub text: Rgba<f32>,
    pub room: Rgba<f32>,
    pub wall: Rgba<f32>,
    pub wall_block: Rgba<f32>,
    pub player: Rgba<f32>,
    pub minion: Rgba<f32>,
    pub object: Rgba<f32>,
    pub enemy: Rgba<f32>,
    pub health: Rgba<f32>,
    pub drawing: Rgba<f32>,
    pub dash: Rgba<f32>,
    pub damage: Rgba<f32>,
    pub collision: Rgba<f32>,
    pub upgrade: Rgba<f32>,
    pub pacman_1up: Rgba<f32>,
    pub idk: Rgba<f32>,
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
    pub upgrades_per_level: usize,
    pub difficulty: DifficultyConfig,
    pub score: ScoreConfig,
    pub player: PlayerConfig,
    pub enemies: HashMap<String, EnemyConfig>,
    pub bosses: Vec<BossConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreConfig {
    pub room_bonus: Score,
    pub upgrade_multiplier: R32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyConfig {
    pub initial: R32,
    pub upgrade_amount: R32,
    pub time_scaling: R32,
    pub room_bonus: R32,
    pub room_exponent: R32,
    pub enemy_health_scaling: R32,
    pub room_size_scaling: R32,
    pub room_size_max: R32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub health: Hp,
    pub speed: Coord,
    pub acceleration: Coord,
    pub hurt_invincibility_time: Time,
    pub whip: DrawConfig,
    pub dash: DrawConfig,
    pub bow: DrawConfig,
    pub fishing: DrawConfig,
    pub shape: Shape,
    pub shield: Shape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyConfig {
    pub cost: Option<R32>,
    pub score: Option<Score>,
    pub grouping: Option<EnemyGrouping>,
    pub mass: Option<R32>,
    pub health: Hp,
    pub damage: Hp,
    pub speed: Coord,
    pub acceleration: Coord,
    pub shape: Shape,
    pub ai: EnemyAI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyGrouping {
    pub cost: R32,
    pub chance: R32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawConfig {
    pub cooldown: Bounded<Time>,
    pub max_distance: Coord,
    pub speed: Coord,
    pub width: Coord,
    pub damage: Hp,
    pub invincibility_time: Time,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossConfig {
    pub room: usize,
    pub room_size: vec2<Coord>,
    pub enemies: Vec<String>,
}

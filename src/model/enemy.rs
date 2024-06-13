use super::*;

#[derive(Debug, Clone)]
pub struct Enemy {
    pub health: Health,
    pub body: PhysicsBody,
    pub stats: EnemyConfig,
    pub ai: EnemyAI,
}

impl Enemy {
    pub fn new(config: EnemyConfig, position: Position) -> Self {
        Self {
            health: Bounded::new_max(config.health),
            body: PhysicsBody::new(position, config.shape),
            ai: config.ai.clone(),
            stats: config,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnemyAI {
    Idle,
    Crawler,
    Shooter {
        preferred_distance: Coord,
        charge: Bounded<Time>,
        bullet: Box<EnemyConfig>,
    },
    Pacman {
        #[serde(default)]
        pacman: PacmanAI,
    },
    Helicopter {
        #[serde(default)]
        helicopter: HelicopterAI,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacmanAI {
    pub state: PacmanState,
    pub speed_power: Coord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PacmanState {
    Normal {
        spawn_1up: Bounded<Time>,
        target: Option<Position>,
    },
    Power {
        timer: Bounded<Time>,
    },
}

impl Default for PacmanAI {
    fn default() -> Self {
        Self {
            state: PacmanState::Normal {
                spawn_1up: Bounded::new_max(r32(5.0)),
                target: None,
            },
            speed_power: r32(9.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pacman1Up {
    pub collider: Collider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelicopterAI {
    pub oscilate: Bounded<Time>,
    pub target: Option<Position>,
}

impl Default for HelicopterAI {
    fn default() -> Self {
        Self {
            oscilate: Bounded::new_max(r32(7.0)),
            target: None,
        }
    }
}

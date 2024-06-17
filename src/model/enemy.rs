use super::*;

#[derive(HasId, Debug, Clone)]
pub struct Enemy {
    pub id: Id,
    pub is_boss: bool,
    pub health: Health,
    pub last_hit: Time,
    pub invincibility: Bounded<Time>,
    pub body: PhysicsBody,
    pub attached_to: Option<(Id, vec2<Coord>)>,
    pub stats: EnemyConfig,
    pub ai: EnemyAI,
}

impl Enemy {
    pub fn new(id: Id, config: EnemyConfig, position: Position) -> Self {
        let mut body = PhysicsBody::new(position, config.shape);
        body.mass = config.mass.unwrap_or(R32::ONE);
        Self {
            id,
            is_boss: false,
            health: Bounded::new_max(config.health),
            last_hit: r32(-999.0),
            invincibility: Bounded::new_zero(r32(0.5)),
            body,
            attached_to: None,
            ai: config.ai.clone(),
            stats: config,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnemyAI {
    Idle,
    Bullet,
    Crawler,
    Shooter {
        preferred_distance: Coord,
        charge: Bounded<Time>,
        bullet: Box<EnemyConfig>,
    },
    Healer {
        range: Coord,
        heal_ratio: R32,
        cooldown: Bounded<Time>,
    },
    Shielder {
        preferred_distance: Coord,
        target: Option<Id>,
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
            speed_power: r32(10.0),
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
    pub state: HelicopterState,
    pub minigun_bullet: Box<EnemyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HelicopterState {
    Idle,
    Moving(Position),
    Minigun {
        timer: Time,
        shot_delay: Bounded<Time>,
    },
    Minions {
        minions: Vec<EnemyConfig>,
        delay: Bounded<Time>,
    },
}

impl Default for HelicopterAI {
    fn default() -> Self {
        Self {
            oscilate: Bounded::new_zero(r32(1.5)),
            state: HelicopterState::Idle,
            minigun_bullet: Box::new(EnemyConfig {
                cost: None,
                score: None,
                grouping: None,
                mass: None,
                health: r32(1.0),
                damage: r32(5.0),
                speed: r32(15.0),
                acceleration: r32(100.0),
                shape: Shape::circle(0.2),
                ai: EnemyAI::Bullet,
            }),
        }
    }
}

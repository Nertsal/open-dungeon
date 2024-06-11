mod collider;
mod logic;
mod particles;

pub use self::{collider::*, particles::*};

use crate::prelude::*;

use std::collections::VecDeque;

pub type Camera = Camera2d;
pub type Coord = R32;
pub type Position = vec2<Coord>;
pub type Time = R32;
pub type Hp = R32;
pub type Health = Bounded<Hp>;

pub struct Model {
    pub config: Config,
    pub camera: Camera,
    pub real_time: Time,
    pub game_time: Time,

    pub player: Player,
    pub objects: Vec<Object>,
    pub enemies: Vec<Enemy>,
    pub particles: Arena<Particle>,

    pub particles_queue: Vec<SpawnParticles>,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub collider: Collider,
}

#[derive(Debug, Clone)]
pub struct PhysicsBody {
    pub collider: Collider,
    pub velocity: vec2<Coord>,
    pub angular_velocity: Angle<R32>,
}

impl PhysicsBody {
    pub fn new(position: Position, shape: Shape) -> Self {
        Self {
            collider: Collider::new(position, shape),
            velocity: vec2::ZERO,
            angular_velocity: Angle::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub health: Health,
    pub body: PhysicsBody,
    pub stats: EnemyConfig,
    pub ai: EnemyAI,
}

#[derive(Debug, Clone)]
pub enum EnemyAI {
    Idle,
    Crawler,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub health: Health,
    pub body: PhysicsBody,
    pub stats: PlayerConfig,
    pub draw_action: Option<Drawing>,
}

#[derive(Debug, Clone)]
pub struct Drawing {
    pub points_raw: VecDeque<DrawPoint>,
    pub points_smoothed: Vec<Position>,
}

#[derive(Debug, Clone)]
pub struct DrawPoint {
    pub position: Position,
    pub time: Time,
}

#[derive(Debug, Clone)]
pub struct PlayerControls {
    pub move_dir: vec2<Coord>,
    pub drawing: Option<vec2<Coord>>,
}

impl Model {
    pub fn new(config: Config) -> Self {
        let enemy_stats = EnemyConfig {
            health: r32(10.0),
            speed: r32(3.0),
            acceleration: r32(10.0),
        };
        Self {
            camera: Camera {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: 20.0,
            },
            real_time: Time::ZERO,
            game_time: Time::ZERO,

            player: Player {
                health: Health::new_max(config.player.health),
                body: PhysicsBody::new(vec2::ZERO, Shape::circle(0.5)),
                stats: config.player.clone(),
                draw_action: None,
            },
            objects: vec![Object {
                collider: Collider::new(vec2(3.0, 2.0).as_r32(), Shape::circle(0.6)),
            }],
            enemies: vec![
                Enemy {
                    health: Health::new_max(enemy_stats.health),
                    body: PhysicsBody::new(vec2(5.0, -3.0).as_r32(), Shape::square(0.4)),
                    stats: enemy_stats.clone(),
                    ai: EnemyAI::Idle,
                },
                Enemy {
                    health: Health::new_max(enemy_stats.health),
                    body: PhysicsBody::new(vec2(3.0, -2.0).as_r32(), Shape::circle(0.4)),
                    stats: enemy_stats.clone(),
                    ai: EnemyAI::Crawler,
                },
            ],
            particles: Arena::new(),

            particles_queue: Vec::new(),

            config,
        }
    }
}

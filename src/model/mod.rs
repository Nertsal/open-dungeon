mod collider;
mod logic;

pub use self::collider::*;

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
}

#[derive(Debug, Clone)]
pub struct Object {
    pub collider: Collider,
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub health: Health,
    pub collider: Collider,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub health: Health,
    pub collider: Collider,
    pub velocity: vec2<Coord>,
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
                collider: Collider::new(vec2::ZERO, Shape::circle(0.5)),
                velocity: vec2::ZERO,
                stats: config.player.clone(),
                draw_action: None,
            },
            objects: vec![Object {
                collider: Collider::new(vec2(3.0, 2.0).as_r32(), Shape::circle(0.6)),
            }],
            enemies: vec![
                Enemy {
                    health: Health::new_max(r32(10.0)),
                    collider: Collider::new(vec2(5.0, -3.0).as_r32(), Shape::square(0.4)),
                },
                Enemy {
                    health: Health::new_max(r32(20.0)),
                    collider: Collider::new(vec2(3.0, -2.0).as_r32(), Shape::circle(0.4)),
                },
            ],

            config,
        }
    }
}

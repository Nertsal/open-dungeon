mod collider;
mod logic;

pub use self::collider::*;

use crate::prelude::*;

use std::collections::VecDeque;

pub type Camera = Camera2d;
pub type Coord = R32;
pub type Position = vec2<Coord>;
pub type Time = R32;

pub struct Model {
    pub camera: Camera,
    pub real_time: Time,
    pub game_time: Time,

    pub player: Player,
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub collider: Collider,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub collider: Collider,
    pub speed: Coord,
    pub draw_action: Option<Drawing>,
}

#[derive(Debug, Clone)]
pub struct Drawing {
    pub points: VecDeque<DrawPoint>,
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
                collider: Collider::new(vec2::ZERO, Shape::circle(0.5)),
                speed: config.player.speed,
                draw_action: None,
            },
            objects: vec![Object {
                collider: Collider::new(vec2(3.0, 2.0).as_r32(), Shape::circle(0.6)),
            }],
        }
    }
}

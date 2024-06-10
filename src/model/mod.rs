mod collider;

pub use self::collider::*;

use crate::prelude::*;

pub type Camera = Camera2d;
pub type Coord = R32;
pub type Position = vec2<Coord>;

pub struct Model {
    pub camera: Camera,
    pub player: Player,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub collider: Collider,
}

impl Model {
    pub fn new() -> Self {
        Self {
            camera: Camera {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: 10.0,
            },
            player: Player {
                collider: Collider::new(vec2::ZERO, Shape::circle(0.5)),
            },
        }
    }
}

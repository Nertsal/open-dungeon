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
    pub particles: Arena<Particle>,
}

#[derive(Debug, Clone)]
pub struct SpawnParticles {
    pub density: R32,
    pub distribution: ParticleDistribution,
    pub size: RangeInclusive<Coord>,
    pub velocity: vec2<Coord>,
    pub lifetime: RangeInclusive<Time>,
}

#[derive(Debug, Clone)]
pub enum ParticleDistribution {
    Circle { center: Position, radius: Coord },
    Drawing { points: Vec<Position>, width: Coord },
}

impl ParticleDistribution {
    pub fn sample(&self, rng: &mut impl Rng, density: R32) -> Vec<Position> {
        match self {
            &ParticleDistribution::Circle { center, radius } => {
                let amount = (density * radius.sqr() * R32::PI).floor().as_f32() as usize;
                (0..amount)
                    .map(|_| rng.gen_circle(center, radius))
                    .collect()
            }
            ParticleDistribution::Drawing { points, width } => points
                .windows(2)
                .flat_map(|segment| {
                    let &[a, b] = segment else { unreachable!() };
                    let amount = (density * (b - a).len() * *width).floor().as_f32() as usize;
                    let ts: Vec<_> = rng
                        .sample_iter(rand::distributions::Uniform::new_inclusive(
                            R32::ZERO,
                            R32::ONE,
                        ))
                        .take(amount)
                        .collect();
                    ts.into_iter().map(move |t| a + (b - a) * t)
                })
                .collect(),
        }
    }
}

impl Default for SpawnParticles {
    fn default() -> Self {
        Self {
            density: r32(5.0),
            distribution: ParticleDistribution::Circle {
                center: vec2::ZERO,
                radius: r32(0.5),
            },
            size: r32(0.05)..=r32(0.15),
            velocity: vec2::ZERO,
            lifetime: r32(0.5)..=r32(1.5),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub collider: Collider,
    pub velocity: vec2<Coord>,
    pub lifetime: Bounded<Time>,
    // pub kind: ParticleKind,
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
            particles: Arena::new(),

            config,
        }
    }
}

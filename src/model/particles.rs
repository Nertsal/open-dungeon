use super::*;

#[derive(Debug, Clone)]
pub struct SpawnParticles {
    pub kind: ParticleKind,
    pub density: R32,
    pub distribution: ParticleDistribution,
    pub size: RangeInclusive<Coord>,
    pub velocity: vec2<Coord>,
    pub lifetime: RangeInclusive<Time>,
}

#[derive(Debug, Clone, Copy)]
pub enum ParticleKind {
    Draw,
    Bounce,
    Damage,
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
                let amount = (density * radius.sqr() * R32::PI).ceil().as_f32() as usize;
                (0..amount)
                    .map(|_| rng.gen_circle(center, radius))
                    .collect()
            }
            ParticleDistribution::Drawing { points, width } => {
                let mut left_out = R32::ZERO;
                points
                    .windows(2)
                    .flat_map(|segment| {
                        let &[a, b] = segment else { unreachable!() };
                        let n = (b - a).normalize_or_zero().rotate_90();

                        let amount = density * (b - a).len() * *width;
                        left_out += amount.fract();
                        let amount = (amount.floor() + left_out.floor()).as_f32() as usize;
                        left_out = left_out.fract();

                        let us: Vec<_> = rng
                            .sample_iter(rand::distributions::Uniform::new_inclusive(
                                -R32::ONE,
                                R32::ONE,
                            ))
                            .take(amount)
                            .collect();
                        let ts: Vec<_> = rng
                            .sample_iter(rand::distributions::Uniform::new_inclusive(
                                R32::ZERO,
                                R32::ONE,
                            ))
                            .zip(us)
                            .take(amount)
                            .collect();
                        ts.into_iter()
                            .map(move |(t, u)| a + (b - a) * t + n * *width * u)
                    })
                    .collect()
            }
        }
    }
}

impl Default for SpawnParticles {
    fn default() -> Self {
        Self {
            kind: ParticleKind::Draw,
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
    pub kind: ParticleKind,
    pub collider: Collider,
    pub velocity: vec2<Coord>,
    pub lifetime: Bounded<Time>,
}

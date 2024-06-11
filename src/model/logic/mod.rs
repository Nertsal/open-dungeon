mod controls;

use super::*;

impl Model {
    pub fn update(&mut self, input: PlayerControls, delta_time: Time) {
        self.real_time += delta_time;
        self.game_time += delta_time;

        self.controls(input, delta_time);
        self.ai(delta_time);
        self.collisions(delta_time);
        self.check_deaths(delta_time);
    }

    pub fn collisions(&mut self, _delta_time: Time) {
        // Player - Object collisions
        let player = &mut self.player;
        for object in &self.objects {
            if let Some(collision) = player.body.collider.collide(&object.collider) {
                player.body.collider.position -= collision.normal * collision.penetration;
                player.body.velocity -=
                    collision.normal * vec2::dot(player.body.velocity, collision.normal);
            }
        }

        // Player - Enemy collisions
        let player = &mut self.player;
        for enemy in &mut self.enemies {
            if let Some(collision) = player.body.collider.collide(&enemy.body.collider) {
                let correction = collision.normal * collision.penetration;

                let player_t = r32(0.5);
                let enemy_t = R32::ONE - player_t;

                let bounciness = r32(2.0);
                let rel_vel = player.body.velocity - enemy.body.velocity;
                let bounce = collision.normal
                    * vec2::dot(rel_vel, collision.normal)
                    * (R32::ONE + bounciness);

                player.body.collider.position -= correction * player_t;
                player.body.velocity -= bounce * player_t;

                enemy.body.collider.position += correction * enemy_t;
                enemy.body.velocity += bounce * enemy_t;

                self.particles_queue.push(SpawnParticles {
                    kind: ParticleKind::Bounce,
                    distribution: ParticleDistribution::Circle {
                        center: collision.point,
                        radius: r32(0.2),
                    },
                    ..default()
                });
            }
        }
    }

    pub fn check_deaths(&mut self, delta_time: Time) {
        self.enemies.retain(|enemy| enemy.health.is_above_min());

        self.particles.retain(|_, particle| {
            particle.lifetime.change(-delta_time);
            particle.lifetime.is_above_min()
        });
        let spawn = self.particles_queue.drain(..).flat_map(spawn_particles);
        self.particles.extend(spawn);
    }

    pub fn ai(&mut self, delta_time: Time) {
        for enemy in &mut self.enemies {
            match &enemy.ai {
                EnemyAI::Idle => {
                    let drag = r32(0.9);
                    enemy.body.velocity *= drag;
                }
                EnemyAI::Crawler => {
                    let target = self.player.body.collider.position;
                    let target_velocity = (target - enemy.body.collider.position)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                }
            }

            enemy.body.collider.position += enemy.body.velocity * delta_time;
            enemy.body.collider.rotation += enemy.body.angular_velocity * delta_time;
        }
    }

    pub fn damage_around(&mut self, drawing: Drawing, width: Coord, base_damage: Hp) {
        for enemy in &mut self.enemies {
            let Some(delta) =
                delta_to_chain(enemy.body.collider.position, &drawing.points_smoothed)
            else {
                continue;
            };

            // TODO: maybe account for collider shape or size
            if delta.len() < width {
                enemy.health.change(-base_damage); // TODO: combo scaling

                let size = enemy.body.collider.compute_aabb().size();
                self.particles_queue.push(SpawnParticles {
                    kind: ParticleKind::Damage,
                    distribution: ParticleDistribution::Circle {
                        center: enemy.body.collider.position,
                        radius: size.len() / r32(2.0),
                    },
                    ..default()
                });
            }
        }

        self.particles_queue.push(SpawnParticles {
            distribution: ParticleDistribution::Drawing {
                points: drawing.points_smoothed.clone(),
                width,
            },
            ..default()
        })
    }
}

fn delta_to_chain(point: Position, chain: &[Position]) -> Option<vec2<Coord>> {
    // NOTE: potentially optimize by storing normal and distance separately
    let mut closest: Option<vec2<Coord>> = None;
    for segment in chain.windows(2) {
        let &[a, b] = segment else { unreachable!() };
        let delta = delta_to_segment(point, (a, b));
        match closest {
            Some(d) if d.len() < delta.len() => {}
            _ => closest = Some(delta),
        }
    }
    closest
}

fn delta_to_segment(point: Position, segment: (Position, Position)) -> vec2<Coord> {
    let delta_pos = point - segment.0;
    let normal = (segment.1 - segment.0).rotate_90().normalize_or_zero();

    // Projection
    let segment_dir = segment.1 - segment.0;
    let t = vec2::dot(delta_pos, segment_dir) / segment_dir.len_sqr();

    if (0.0..=1.0).contains(&t.as_f32()) {
        let dot = vec2::dot(normal, delta_pos);
        normal * dot
    } else if t < Coord::ZERO {
        segment.0 - point
    } else {
        segment.1 - point
    }
}

fn spawn_particles(options: SpawnParticles) -> impl Iterator<Item = Particle> {
    let mut rng = thread_rng();
    options
        .distribution
        .sample(&mut rng, options.density)
        .into_iter()
        .map(move |position| {
            let velocity = rng.gen_circle(options.velocity, r32(0.2));
            let size = rng.gen_range(options.size.clone());
            let lifetime = rng.gen_range(options.lifetime.clone());
            Particle {
                kind: options.kind,
                collider: Collider::new(position, Shape::circle(size)),
                velocity,
                lifetime: Bounded::new_max(lifetime),
            }
        })
}
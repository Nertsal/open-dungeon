mod controls;

use std::collections::BTreeMap;

use super::*;

impl Model {
    pub fn update(&mut self, input: PlayerControls, delta_time: Time) {
        self.real_time += delta_time;
        self.game_time += delta_time;

        if self.player.draw_action.is_some() {
            self.events.push(Event::Sound(SoundEvent::Drawing));
        }

        self.controls(input, delta_time);
        self.ai(delta_time);
        self.collisions(delta_time);
        self.passive_particles(delta_time);
        self.check_deaths(delta_time);
        self.update_camera(delta_time);
    }

    pub fn can_expand(&self) -> bool {
        self.enemies.is_empty()
    }

    pub fn passive_particles(&mut self, _delta_time: Time) {
        let kind = if self.can_expand() {
            ParticleKind::WallBreakable
        } else {
            ParticleKind::WallBlock
        };
        for collider in &self.room_colliders {
            self.particles_queue.push(SpawnParticles {
                kind,
                density: r32(0.005),
                distribution: ParticleDistribution::Aabb(collider.compute_aabb()),
                lifetime: r32(2.0)..=r32(3.0),
                ..default()
            });
        }
    }

    pub fn update_camera(&mut self, delta_time: Time) {
        if self.rooms.contains(Index::from_raw_parts(0, 0)) && self.rooms.len() == 1
            || self.player.draw_action.is_some()
        {
            return;
        }

        let offset = (self.player.body.collider.position - self.camera.center.as_r32()) / r32(0.5)
            * delta_time;
        self.camera.center += offset.as_f32();
    }

    pub fn collisions(&mut self, _delta_time: Time) {
        // Player - Object collisions
        let player = &mut self.player;
        for object in &self.objects {
            if let Some(collision) = player.body.collider.collide(&object.collider) {
                if player.body.velocity.len_sqr() > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }

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

                let player_t = if player.draw_action.is_some() {
                    r32(0.0)
                } else {
                    r32(0.5)
                };
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
                if rel_vel.len_sqr() > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }
            }
        }

        // Room collisions
        let player = &mut self.player;
        for room in &self.room_colliders {
            if let Some(collision) = player.body.collider.collide(room) {
                if player.body.velocity.len_sqr() > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }

                let bounciness = r32(0.8);
                player.body.collider.position -= collision.normal * collision.penetration;
                player.body.velocity -= collision.normal
                    * vec2::dot(player.body.velocity, collision.normal)
                    * (Coord::ONE + bounciness);
            }

            for enemy in &mut self.enemies {
                if let Some(collision) = enemy.body.collider.collide(room) {
                    if enemy.body.velocity.len_sqr() > r32(1.0) {
                        self.events.push(Event::Sound(SoundEvent::Bounce));
                    }

                    let bounciness = r32(0.8);
                    enemy.body.collider.position -= collision.normal * collision.penetration;
                    enemy.body.velocity -= collision.normal
                        * vec2::dot(enemy.body.velocity, collision.normal)
                        * (Coord::ONE + bounciness);
                }
            }
        }
    }

    pub fn check_deaths(&mut self, delta_time: Time) {
        self.enemies.retain(|enemy| {
            let alive = enemy.health.is_above_min();
            if !alive {
                self.events.push(Event::Sound(SoundEvent::Kill));
            }
            alive
        });

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
                self.events.push(Event::Sound(SoundEvent::Hit));
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

    pub fn update_room_colliders(&mut self) {
        struct Sides {
            room: Aabb2<Coord>,
            left: Vec<(Coord, Coord)>,
            right: Vec<(Coord, Coord)>,
            bottom: Vec<(Coord, Coord)>,
            top: Vec<(Coord, Coord)>,
        }

        fn open(side: &mut Vec<(Coord, Coord)>, segment: (Coord, Coord)) {
            for (i, (a, b)) in side.iter_mut().enumerate() {
                let in0 = (*a..=*b).contains(&segment.0);
                let in1 = (*a..=*b).contains(&segment.1);
                if in0 {
                    if in1 {
                        let end = *b;
                        *b = segment.0;
                        side.insert(i + 1, (segment.1, end));
                        break;
                    } else {
                        *b = segment.0;
                    }
                } else if in1 {
                    *a = segment.1;
                }
            }
            side.retain(|(a, b)| b > a);
        }

        let mut all_sides: BTreeMap<Index, Sides> = self
            .rooms
            .iter()
            .map(|(idx, room)| {
                (
                    idx,
                    Sides {
                        room: room.area,
                        left: vec![(room.area.min.y, room.area.max.y)],
                        right: vec![(room.area.min.y, room.area.max.y)],
                        bottom: vec![(room.area.min.x, room.area.max.x)],
                        top: vec![(room.area.min.x, room.area.max.x)],
                    },
                )
            })
            .collect();

        for (idx, room) in &self.rooms {
            if let Some(prev_idx) = room.unlocked_after {
                if let Some(prev_room) = self.rooms.get(prev_idx) {
                    let room = room.area;
                    let prev = prev_room.area;
                    let sides = all_sides.get_mut(&prev_idx).unwrap();
                    if prev.max.x == room.min.x {
                        // Right side
                        let intersection = (room.min.y.max(prev.min.y), room.max.y.min(prev.max.y));
                        open(&mut sides.right, intersection);
                        open(&mut all_sides.get_mut(&idx).unwrap().left, intersection);
                    } else if prev.min.x == room.max.x {
                        // Left side
                        let intersection = (room.min.y.max(prev.min.y), room.max.y.min(prev.max.y));
                        open(&mut sides.left, intersection);
                        open(&mut all_sides.get_mut(&idx).unwrap().right, intersection);
                    } else if prev.max.y == room.min.y {
                        // Top side
                        let intersection = (room.min.x.max(prev.min.x), room.max.x.min(prev.max.x));
                        open(&mut sides.top, intersection);
                        open(&mut all_sides.get_mut(&idx).unwrap().bottom, intersection);
                    } else if prev.min.y == room.max.y {
                        // Bottom side
                        let intersection = (room.min.x.max(prev.min.x), room.max.x.min(prev.max.x));
                        open(&mut sides.bottom, intersection);
                        open(&mut all_sides.get_mut(&idx).unwrap().top, intersection);
                    } else {
                        unreachable!("invalid room setup")
                    }
                }
            }
        }

        let width = r32(0.1);
        let wall_vert = |x, (y_min, y_max)| {
            Collider::aabb(
                Aabb2::point(vec2(x, y_min))
                    .extend_up(y_max - y_min)
                    .extend_symmetric(vec2(width, Coord::ZERO) / r32(2.0)),
            )
        };
        let wall_horiz = |y, (x_min, x_max)| {
            Collider::aabb(
                Aabb2::point(vec2(x_min, y))
                    .extend_right(x_max - x_min)
                    .extend_symmetric(vec2(Coord::ZERO, width) / r32(2.0)),
            )
        };

        let colliders = all_sides
            .into_values()
            .flat_map(|sides| {
                sides
                    .left
                    .into_iter()
                    .map(move |segment| wall_vert(sides.room.min.x, segment))
                    .chain(
                        sides
                            .right
                            .into_iter()
                            .map(move |segment| wall_vert(sides.room.max.x, segment)),
                    )
                    .chain(
                        sides
                            .bottom
                            .into_iter()
                            .map(move |segment| wall_horiz(sides.room.min.y, segment)),
                    )
                    .chain(
                        sides
                            .top
                            .into_iter()
                            .map(move |segment| wall_horiz(sides.room.max.y, segment)),
                    )
            })
            .collect();
        self.room_colliders = colliders;
    }

    pub fn unlock_room(&mut self, room_idx: Index, pos: Position) {
        let mut rng = thread_rng();
        let Some(room) = self.rooms.get(room_idx) else {
            return;
        };

        if room.area.contains(pos) {
            return;
        }

        let mut dist = r32(9999999999.0);
        let mut closest = Direction::Left;
        let left = room.area.min.x - pos.x;
        if left > Coord::ZERO && left < dist {
            dist = left;
            closest = Direction::Left;
        }
        let right = pos.x - room.area.max.x;
        if right > Coord::ZERO && right < dist {
            dist = right;
            closest = Direction::Right;
        }
        let bottom = room.area.min.y - pos.y;
        if bottom > Coord::ZERO && bottom < dist {
            dist = bottom;
            closest = Direction::Down;
        }
        let top = pos.y - room.area.max.y;
        if top > Coord::ZERO && top < dist {
            // dist = top;
            closest = Direction::Up;
        }

        let size = vec2(rng.gen_range(15.0..=25.0), rng.gen_range(15.0..=25.0)).as_r32();
        let new_room = match closest {
            Direction::Left => Aabb2::point(vec2(room.area.min.x, room.area.center().y))
                .extend_left(size.x)
                .extend_symmetric(vec2(Coord::ZERO, size.y) / r32(2.0)),
            Direction::Right => Aabb2::point(vec2(room.area.max.x, room.area.center().y))
                .extend_right(size.x)
                .extend_symmetric(vec2(Coord::ZERO, size.y) / r32(2.0)),
            Direction::Down => Aabb2::point(vec2(room.area.center().x, room.area.min.y))
                .extend_down(size.y)
                .extend_symmetric(vec2(size.x, Coord::ZERO) / r32(2.0)),
            Direction::Up => Aabb2::point(vec2(room.area.center().x, room.area.max.y))
                .extend_up(size.y)
                .extend_symmetric(vec2(size.x, Coord::ZERO) / r32(2.0)),
        };
        let new_room = self.rooms.insert(Room {
            area: new_room,
            unlocked_after: Some(room_idx),
        });
        self.update_room_colliders();
        self.spawn_enemies(new_room);
        self.events.push(Event::Sound(SoundEvent::Expand));
    }

    pub fn spawn_enemies(&mut self, room_idx: Index) {
        let Some(room) = self.rooms.get(room_idx) else {
            return;
        };

        let mut rng = thread_rng();
        let mut difficulty = r32(5.0); // TODO dynamic
        while let Some(config) = self
            .config
            .enemies
            .iter()
            .filter(|config| config.cost <= difficulty)
            .choose(&mut rng)
        {
            for _ in 0..50 {
                let position = vec2(
                    rng.gen_range(room.area.min.x..=room.area.max.x),
                    rng.gen_range(room.area.min.y..=room.area.max.y),
                );
                if (self.player.body.collider.position - position).len() < r32(5.0) {
                    continue;
                }

                difficulty -= config.cost;
                self.enemies.push(Enemy {
                    health: Bounded::new_max(config.health),
                    body: PhysicsBody::new(position, config.shape),
                    ai: config.ai.clone(),
                    stats: config.clone(),
                });
                break;
            }
        }
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

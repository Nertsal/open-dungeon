mod controls;

use std::collections::BTreeMap;

use super::*;

impl Model {
    pub fn update(&mut self, input: PlayerControls, delta_time: Time) {
        self.real_time += delta_time;
        self.game_time += delta_time;
        self.difficulty += self.config.difficulty.time_scaling * delta_time;

        if self.player.draw_action.is_some() {
            self.events.push(Event::Sound(SoundEvent::Drawing));
        }

        self.compress_rooms(delta_time);
        self.controls(input, delta_time);
        self.ai(delta_time);
        self.collisions(delta_time);
        self.passive_particles(delta_time);
        self.check_deaths(delta_time);
        self.update_camera(delta_time);
        self.process_spawns(delta_time);
    }

    pub fn can_expand(&self) -> bool {
        self.enemies.is_empty()
    }

    pub fn process_spawns(&mut self, _delta_time: Time) {
        let spawns = std::mem::take(&mut self.spawn_queue);
        self.enemies.extend(spawns);
    }

    pub fn passive_particles(&mut self, _delta_time: Time) {
        let can_expand = self.can_expand();
        for (index, direction, collider) in &self.room_colliders {
            let can_expand = can_expand
                && self.rooms.get(*index).map_or(false, |room| {
                    room.expanded_direction.is_none()
                        && room
                            .unlocked_after
                            .map_or(true, |(_, dir)| dir != *direction)
                });
            let kind = if can_expand {
                ParticleKind::WallBreakable
            } else {
                ParticleKind::WallBlock
            };
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
                player.body.collider.position -= collision.normal * collision.penetration;
                let projection = vec2::dot(player.body.velocity, collision.normal);
                player.body.velocity -= collision.normal * projection;
                if projection > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }
            }
        }

        // Player - Enemy collisions
        let player = &mut self.player;
        let player_shield = Collider::new(player.body.collider.position, player.stats.shield);
        for enemy in &mut self.enemies {
            let (damage_mult, player_collider) = if player.invincibility.is_above_min() {
                (Hp::ZERO, &player_shield)
            } else {
                (Hp::ONE, &player.body.collider)
            };
            if let Some(collision) = player_collider.collide(&enemy.body.collider) {
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
                let damage = enemy.stats.damage * damage_mult;
                player.health.change(-damage);
                if damage > Hp::ZERO {
                    player
                        .invincibility
                        .set(player.stats.hurt_invincibility_time);
                    self.particles_queue.push(SpawnParticles {
                        kind: ParticleKind::HitSelf,
                        distribution: ParticleDistribution::Circle {
                            center: player.body.collider.position,
                            radius: r32(0.6),
                        },
                        ..default()
                    });
                    self.events.push(Event::Sound(SoundEvent::HitSelf));
                }

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
        for (_, _, room) in &self.room_colliders {
            if let Some(collision) = player.body.collider.collide(room) {
                let bounciness = r32(0.8);
                player.body.collider.position -= collision.normal * collision.penetration;
                let projection = vec2::dot(player.body.velocity, collision.normal);
                player.body.velocity -= collision.normal * projection * (Coord::ONE + bounciness);
                if projection > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }
            }

            for enemy in &mut self.enemies {
                if let Some(collision) = enemy.body.collider.collide(room) {
                    let bounciness = r32(0.8);
                    enemy.body.collider.position -= collision.normal * collision.penetration;
                    let projection = vec2::dot(enemy.body.velocity, collision.normal);
                    enemy.body.velocity -=
                        collision.normal * projection * (Coord::ONE + bounciness);
                    if projection > r32(1.0) {
                        self.events.push(Event::Sound(SoundEvent::Bounce));
                    }
                }
            }
        }
    }

    pub fn collect_upgrade(&mut self, upgrade: Upgrade) {
        match upgrade.effect {
            UpgradeEffect::Width => self.player.stats.dash.width += r32(0.5),
            UpgradeEffect::Range => self.player.stats.dash.max_distance += r32(3.0),
            UpgradeEffect::Damage => self.player.stats.dash.damage += r32(5.0),
            UpgradeEffect::Speed => {
                self.player.stats.speed += r32(2.0);
                self.player.stats.acceleration += r32(5.0);
            }
            UpgradeEffect::Difficulty => {
                self.difficulty += self.config.difficulty.upgrade_amount;
                self.score_multiplier += self.config.score.upgrade_multiplier;
            }
        };
        self.particles_queue.push(SpawnParticles {
            kind: ParticleKind::Upgrade,
            distribution: ParticleDistribution::Circle {
                center: upgrade.collider.position,
                radius: upgrade.collider.compute_aabb().size().len(),
            },
            ..default()
        });
    }

    pub fn check_deaths(&mut self, delta_time: Time) {
        let in_battle = !self.enemies.is_empty();
        self.enemies.retain(|enemy| {
            let alive = enemy.health.is_above_min();
            if !alive {
                self.score += (enemy.stats.score.unwrap_or(0) as f32
                    * self.score_multiplier.as_f32()) as Score;
                self.events.push(Event::Sound(SoundEvent::Kill));
            }
            alive
        });
        if in_battle && self.enemies.is_empty() && self.player.health.is_above_min() {
            self.finish_battle();
        }

        self.particles.retain(|_, particle| {
            particle.lifetime.change(-delta_time);
            particle.lifetime.is_above_min()
        });
        let spawn = self.particles_queue.drain(..).flat_map(spawn_particles);
        self.particles.extend(spawn);
    }

    pub fn finish_battle(&mut self) {
        let Some((_, room)) = self
            .rooms
            .iter()
            .find(|(_, room)| room.area.contains(self.player.body.collider.position))
        else {
            return;
        };
        let offset = if room.area.size().aspect() > r32(0.5) {
            vec2(2.5, 0.0).as_r32()
        } else {
            vec2(0.0, 2.5).as_r32()
        };

        self.pacman_1ups.clear();

        let mut rng = thread_rng();
        let options = [
            UpgradeEffect::Width,
            UpgradeEffect::Range,
            UpgradeEffect::Damage,
            UpgradeEffect::Speed,
            UpgradeEffect::Difficulty,
        ];
        let options: Vec<_> = options.choose_multiple(&mut rng, 2).collect();
        let upgrades = options.iter().enumerate().map(|(i, effect)| Upgrade {
            collider: Collider::new(
                room.area.center() + offset * r32(i as f32 - (options.len() as f32 - 1.0) / 2.0),
                Shape::circle(0.5),
            ),
            effect: (**effect).clone(),
        });
        self.upgrades.extend(upgrades);

        self.rooms_cleared += 1;
        self.difficulty += self.config.difficulty.room_bonus
            * self
                .config
                .difficulty
                .room_exponent
                .powf(r32(self.rooms_cleared as f32));
        self.score +=
            (self.config.score.room_bonus as f32 * self.score_multiplier.as_f32()) as Score;
    }

    pub fn ai(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        for enemy in &mut self.enemies {
            match &mut enemy.ai {
                EnemyAI::Idle => {
                    let drag = r32(0.9);
                    enemy.body.velocity *= drag;
                    enemy.move_rotation();
                }
                EnemyAI::Crawler => {
                    let target = self.player.body.collider.position;
                    let target_velocity = (target - enemy.body.collider.position)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                    enemy.move_rotation();
                }
                EnemyAI::Shooter {
                    preferred_distance,
                    charge,
                    bullet,
                } => {
                    charge.change(delta_time);
                    if charge.is_max() {
                        charge.set_ratio(Time::ZERO);
                        self.spawn_queue.push(Enemy::new(
                            EnemyConfig {
                                health: bullet.health
                                    + self.config.difficulty.enemy_health_scaling * self.difficulty,
                                ..(**bullet).clone()
                            },
                            enemy.body.collider.position,
                        ));
                    }

                    let target = self.player.body.collider.position
                        + (enemy.body.collider.position - self.player.body.collider.position)
                            .normalize_or_zero()
                            * *preferred_distance;
                    let target_velocity = (target - enemy.body.collider.position)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                    enemy.move_rotation();
                }
                EnemyAI::Pacman { pacman } => match &mut pacman.state {
                    PacmanState::Normal { spawn_1up, target } => {
                        if let Some((_, room)) = self
                            .rooms
                            .iter()
                            .find(|(_, room)| room.area.contains(enemy.body.collider.position))
                        {
                            // Move to the target
                            let target = match *target {
                                Some(target)
                                    if (target - enemy.body.collider.position).len_sqr()
                                        > r32(1.0) =>
                                {
                                    target
                                }
                                _ => {
                                    // Change target
                                    if let Some(up) = self.pacman_1ups.iter().choose(&mut rng) {
                                        *target = Some(up.collider.position);
                                    } else {
                                        for _ in 0..10 {
                                            let area = room.area.extend_uniform(r32(-5.0));
                                            let position = vec2(
                                                rng.gen_range(area.min.x..=area.max.x),
                                                rng.gen_range(area.min.y..=area.max.y),
                                            );
                                            if (enemy.body.collider.position - position).len_sqr()
                                                > r32(100.0)
                                            {
                                                *target = Some(position);
                                                break;
                                            }
                                        }
                                    }
                                    target.unwrap_or(self.player.body.collider.position)
                                }
                            };
                            let delta = target - enemy.body.collider.position;
                            enemy.body.velocity = if enemy.body.velocity.y == Coord::ZERO
                                && delta.x.abs() > r32(0.5)
                                || enemy.body.velocity.x == Coord::ZERO && delta.y.abs() < r32(0.1)
                            {
                                vec2(delta.x.signum(), Coord::ZERO) * enemy.stats.speed
                            } else {
                                vec2(Coord::ZERO, delta.y.signum()) * enemy.stats.speed
                            };
                            enemy.body.angular_velocity = Angle::ZERO;
                            enemy.body.collider.rotation = enemy.body.velocity.arg();

                            // Spawn 1up
                            spawn_1up.change(-delta_time);
                            if spawn_1up.is_min() {
                                spawn_1up.set_ratio(Time::ONE);

                                let target = room.area.center()
                                    + (room.area.center() - enemy.body.collider.position)
                                        .map(Coord::signum)
                                        * (room.area.size() / r32(2.0) - vec2(10.0, 10.0).as_r32());
                                for _ in 0..10 {
                                    let position = rng.gen_circle(target, r32(3.0));
                                    if !self.pacman_1ups.iter().any(|up| {
                                        (up.collider.position - position).len_sqr() < r32(16.0)
                                    }) {
                                        self.pacman_1ups.push(Pacman1Up {
                                            collider: Collider::new(position, Shape::circle(0.5)),
                                        });
                                        break;
                                    }
                                }
                            }

                            // Eat 1up
                            self.pacman_1ups.retain(|up| {
                                let eat = up.collider.check(&enemy.body.collider);
                                if eat {
                                    pacman.state = PacmanState::Power {
                                        timer: Bounded::new_max(r32(5.0)),
                                    };
                                }
                                !eat
                            });
                        }
                    }
                    PacmanState::Power { timer } => {
                        // Chase the player
                        let delta =
                            self.player.body.collider.position - enemy.body.collider.position;
                        enemy.body.velocity = if enemy.body.velocity.y == Coord::ZERO
                            && delta.x.abs() > r32(0.5)
                            || enemy.body.velocity.x == Coord::ZERO && delta.y.abs() < r32(0.1)
                        {
                            vec2(delta.x.signum(), Coord::ZERO) * pacman.speed_power
                        } else {
                            vec2(Coord::ZERO, delta.y.signum()) * pacman.speed_power
                        };
                        enemy.body.angular_velocity = Angle::ZERO;
                        enemy.body.collider.rotation = enemy.body.velocity.arg();

                        // Update timer
                        timer.change(-delta_time);
                        if timer.is_min() {
                            pacman.state = PacmanState::Normal {
                                spawn_1up: Bounded::new_max(r32(7.0)),
                                target: None,
                            };
                        }
                    }
                },
                EnemyAI::Helicopter { helicopter } => {
                    if let Some((_, room)) = self
                        .rooms
                        .iter()
                        .find(|(_, room)| room.area.contains(enemy.body.collider.position))
                    {
                        // Oscilate
                        helicopter.oscilate.change(-delta_time);
                        if helicopter.oscilate.is_min() || helicopter.target.is_none() {
                            helicopter.oscilate.set_ratio(Time::ONE);
                            // Change target
                            let points = room
                                .area
                                .extend_uniform(-r32(5.0))
                                .corners()
                                .into_iter()
                                .filter(|pos| {
                                    (*pos - enemy.body.collider.position).len_sqr() > r32(1.0)
                                });
                            if let Some(point) = points.choose(&mut rng) {
                                helicopter.target = Some(point);
                            }
                        }

                        // Move to target
                        if let Some(target) = helicopter.target {
                            let delta = target - enemy.body.collider.position;
                            let target_velocity = delta.clamp_len(..=enemy.stats.speed);
                            let acc =
                                if vec2::dot(target_velocity, enemy.body.velocity) < Coord::ZERO {
                                    enemy.stats.acceleration * r32(2.0)
                                } else {
                                    enemy.stats.acceleration
                                };
                            enemy.body.velocity += (target_velocity - enemy.body.velocity)
                                .clamp_len(..=acc * delta_time);
                        }
                    }
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

        // Upgrades
        let mut collected_idx = Vec::new();
        for (i, upgrade) in self.upgrades.iter().enumerate() {
            let Some(delta) = delta_to_chain(upgrade.collider.position, &drawing.points_smoothed)
            else {
                continue;
            };
            if delta.len() < width {
                collected_idx.push((i, delta.len()));
            }
        }
        if let Some((i, _)) = collected_idx.into_iter().min_by_key(|(_, d)| *d) {
            let collected = self.upgrades.swap_remove(i);
            self.collect_upgrade(collected);
            self.upgrades.clear();
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
            if let Some((prev_idx, _)) = room.unlocked_after {
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

        let colliders =
            all_sides
                .into_iter()
                .flat_map(|(idx, sides)| {
                    sides
                        .left
                        .into_iter()
                        .map(move |segment| (Direction::Left, wall_vert(sides.room.min.x, segment)))
                        .chain(sides.right.into_iter().map(move |segment| {
                            (Direction::Right, wall_vert(sides.room.max.x, segment))
                        }))
                        .chain(sides.bottom.into_iter().map(move |segment| {
                            (Direction::Down, wall_horiz(sides.room.min.y, segment))
                        }))
                        .chain(sides.top.into_iter().map(move |segment| {
                            (Direction::Up, wall_horiz(sides.room.max.y, segment))
                        }))
                        .map(move |(dir, collider)| (idx, dir, collider))
                })
                .collect();
        self.room_colliders = colliders;
    }

    pub fn unlock_room(&mut self, room_idx: Index, pos: Position) {
        let mut rng = thread_rng();
        let Some(room) = self.rooms.get_mut(room_idx) else {
            return;
        };

        if room.area.contains(pos) {
            return;
        }

        let closest = room.closest_wall(pos).1;
        // if room.unlocked_after.map_or(false, |(_, dir)| dir == closest) {
        //     return;
        // }

        room.expanded_direction = Some(closest);

        let size = if let Some(boss) = self
            .config
            .bosses
            .iter()
            .find(|boss| boss.room == self.rooms_cleared + 1)
        {
            // Boss room
            log::debug!("Generating boss room...");
            boss.room_size
        } else {
            // Normal room
            log::debug!("Generating next room...");
            let mut gen = || {
                (r32(rng.gen_range(15.0..=25.0))
                    + self.config.difficulty.room_size_scaling * self.difficulty)
                    .min(self.config.difficulty.room_size_max)
            };
            vec2(gen(), gen()).as_r32()
        };

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
        log::debug!("Expanding room near {room_idx:?}, {closest:?}");
        let new_room = self.rooms.insert(Room {
            area: new_room,
            unlocked_after: Some((room_idx, closest.opposite())),
            expanded_direction: None,
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
        let mut difficulty = self.difficulty;

        let mut spawn_enemy = |config: &EnemyConfig, difficulty: &mut R32, rng: &mut ThreadRng| {
            for _ in 0..50 {
                let position = vec2(
                    rng.gen_range(room.area.min.x..=room.area.max.x),
                    rng.gen_range(room.area.min.y..=room.area.max.y),
                );
                if (self.player.body.collider.position - position).len() < r32(5.0) {
                    continue;
                }

                *difficulty -= config.cost.unwrap_or(R32::ZERO);
                self.enemies.push(Enemy::new(
                    EnemyConfig {
                        health: config.health
                            + self.config.difficulty.enemy_health_scaling * self.difficulty,
                        ..config.clone()
                    },
                    position,
                ));
                break;
            }
        };

        if let Some(boss) = self
            .config
            .bosses
            .iter()
            .find(|boss| boss.room == self.rooms_cleared + 1)
        {
            // Boss room
            for enemy in &boss.enemies {
                spawn_enemy(enemy, &mut difficulty, &mut rng);
            }
            return;
        }

        while let Some(config) = self
            .config
            .enemies
            .iter()
            .filter(|config| config.cost.map_or(false, |cost| cost <= difficulty))
            .choose(&mut rng)
        {
            spawn_enemy(config, &mut difficulty, &mut rng);
        }
    }

    pub fn compress_rooms(&mut self, delta_time: Time) {
        if self.can_expand() {
            // Pause for expansion
            return;
        }

        if self.rooms.len() == 1
            && self
                .config
                .bosses
                .iter()
                .any(|boss| boss.room == self.rooms_cleared + 1)
        {
            // Dont compress the boss room
            return;
        }

        let speed = r32(1.5) * r32(self.rooms.len() as f32).powf(r32(1.2));
        let ids: Vec<_> = self.rooms.iter().map(|(idx, _)| idx).collect();
        for (_, room) in &mut self.rooms {
            let dir = room
                .unlocked_after
                .map_or(true, |(idx, _)| !ids.contains(&idx))
                .then(|| {
                    room.expanded_direction
                        .or(room.unlocked_after.map(|(_, dir)| dir.opposite()))
                })
                .flatten();
            if let Some(dir) = dir {
                let shift = speed * delta_time;
                match dir {
                    Direction::Right => room.area.min.x += shift,
                    Direction::Left => room.area.max.x -= shift,
                    Direction::Up => room.area.min.y += shift,
                    Direction::Down => room.area.max.y -= shift,
                }
            }
        }

        let min = r32(0.01);
        let squashed: Vec<_> = self
            .rooms
            .iter()
            .filter(|(_, room)| room.area.width() <= min || room.area.height() <= min)
            .map(|(idx, _)| idx)
            .collect();
        if squashed.is_empty() {
            self.update_room_colliders();
        } else {
            self.squash_rooms(&squashed);
        }
    }

    pub fn squash_rooms(&mut self, ids: &[Index]) {
        if ids.is_empty() {
            return;
        }

        let should_squash = |pos| {
            self.rooms
                .iter()
                .find(|(_, room)| room.area.contains(pos))
                .map_or(true, |(idx, _)| ids.contains(&idx))
        };

        let player = &mut self.player;
        if should_squash(player.body.collider.position) {
            player.health.set_ratio(Hp::ZERO);
        }

        for enemy in &mut self.enemies {
            if should_squash(enemy.body.collider.position) {
                enemy.health.set_ratio(Hp::ZERO);
            }
        }

        self.rooms.retain(|idx, _| !ids.contains(&idx));
        log::debug!("Squashed rooms {ids:?}");
        self.update_room_colliders();
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

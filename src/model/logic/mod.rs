mod controls;

use std::collections::BTreeMap;

use super::*;

impl Model {
    pub fn update(&mut self, input: PlayerControls, delta_time: Time) {
        self.real_time += delta_time;
        self.game_time += delta_time;

        if !self.rooms.contains(Index::from_raw_parts(0, 0)) || self.rooms.len() > 1 {
            // Exited the starting room
            self.difficulty_raw += self.config.difficulty.time_scaling * delta_time;
            let difficulty_step = r32(1.0);
            self.difficulty = (self.difficulty_raw / difficulty_step).floor() * difficulty_step;
        }

        if self.player.draw_action.is_some() {
            self.events.push(Event::Sound(SoundEvent::Drawing));
        }

        self.compress_rooms(delta_time);
        self.controls(input, delta_time);
        self.enemy_ai(delta_time);
        self.minion_ai(delta_time);
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

    pub fn passive_particles(&mut self, delta_time: Time) {
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
                density: r32(50.0) * delta_time,
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
        // Object collisions
        for object in &mut self.objects {
            let player = &mut self.player;
            if let Some(collision) = player.body.collider.collide(&object.collider) {
                player.body.collider.position -= collision.normal * collision.penetration;
                let projection = vec2::dot(player.body.velocity, collision.normal);
                player.body.velocity -= collision.normal * projection;
                if projection > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }
            }

            for minion in &mut self.minions {
                if minion.body.collider.check(&object.collider) {
                    match (&minion.ai, &object.kind) {
                        (MinionAI::Bullet { .. }, ObjectKind::ExplosiveBarrel { .. }) => {
                            // NOTE: explosion managed on death
                            minion.health.set_ratio(Hp::ZERO);
                            object.dead = true;
                            self.events.push(Event::Sound(SoundEvent::Hit));
                        }
                    }
                }
            }

            for enemy in &mut self.enemies {
                if enemy.body.collider.check(&object.collider) {
                    match &object.kind {
                        ObjectKind::ExplosiveBarrel { .. } => {
                            // NOTE: explosion managed on death
                            object.dead = true;
                            self.events.push(Event::Sound(SoundEvent::Hit));
                        }
                    }
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
                    enemy.body.mass / (enemy.body.mass + player.body.mass)
                };
                let enemy_t = R32::ONE - player_t;

                let bounciness = r32(2.0);
                let rel_vel = player.body.velocity - enemy.body.velocity;
                let projection = vec2::dot(rel_vel, collision.normal).max(R32::ZERO);
                let bounce = collision.normal * projection * (R32::ONE + bounciness);

                player.body.collider.position -= correction * player_t;
                player.body.velocity -= bounce * player_t;
                let damage = enemy.stats.damage * damage_mult;
                player.health.change(-damage);
                player.last_hit = self.game_time;
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

                if let EnemyAI::Bullet = enemy.ai {
                    enemy.health.set_ratio(Hp::ZERO);
                }

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

        // Minion - Enemy collisions
        for minion in &mut self.minions {
            for enemy in &mut self.enemies {
                if minion.body.collider.check(&enemy.body.collider) {
                    match minion.ai {
                        MinionAI::Bullet { damage, .. } => {
                            // NOTE: explosion managed on death
                            minion.health.set_ratio(Hp::ZERO);
                            if enemy.invincibility.is_min() {
                                enemy.health.change(-damage);
                            }
                            self.events.push(Event::Sound(SoundEvent::Hit));
                        }
                    }
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
                if projection > r32(0.0) {
                    player.body.velocity -=
                        collision.normal * projection * (Coord::ONE + bounciness);
                }
                if projection > r32(1.0) {
                    self.events.push(Event::Sound(SoundEvent::Bounce));
                }
            }

            for minion in &mut self.minions {
                if minion.body.collider.check(room) {
                    let MinionAI::Bullet { .. } = minion.ai;
                    {
                        minion.health.set_ratio(Hp::ZERO);
                    }

                    // let bounciness = r32(0.8);
                    // minion.body.collider.position -= collision.normal * collision.penetration;
                    // let projection = vec2::dot(minion.body.velocity, collision.normal);
                    // minion.body.velocity -=
                    //     collision.normal * projection * (Coord::ONE + bounciness);
                    // if projection > r32(1.0) {
                    //     self.events.push(Event::Sound(SoundEvent::Bounce));
                    // }
                }
            }

            for enemy in &mut self.enemies {
                if let Some(collision) = enemy.body.collider.collide(room) {
                    let bounciness = r32(0.8);
                    enemy.body.collider.position -= collision.normal * collision.penetration;
                    let projection = vec2::dot(enemy.body.velocity, collision.normal);
                    if projection > r32(0.0) {
                        enemy.body.velocity -=
                            collision.normal * projection * (Coord::ONE + bounciness);
                    }
                    if projection > r32(1.0) {
                        self.events.push(Event::Sound(SoundEvent::Bounce));
                    }

                    if let EnemyAI::Bullet = enemy.ai {
                        enemy.health.set_ratio(Hp::ZERO);
                    }
                }
            }
        }
    }

    pub fn collect_upgrade(&mut self, upgrade: Upgrade) {
        match upgrade.effect {
            UpgradeEffect::Width => {
                self.player.stats.whip.width += r32(0.5);
                self.player.stats.dash.width += r32(0.5);
                self.player.stats.bow.width += r32(0.2);
            }
            UpgradeEffect::Range => {
                self.player.stats.whip.max_distance += r32(3.0);
                self.player.stats.dash.max_distance += r32(3.0);
                self.player.stats.bow.max_distance += r32(3.0);
            }
            UpgradeEffect::Damage => {
                self.player.stats.whip.damage += r32(3.0);
                self.player.stats.dash.damage += r32(3.0);
                self.player.stats.bow.damage += r32(3.0);
            }
            UpgradeEffect::Speed => {
                self.player.stats.speed += r32(1.0);
                self.player.stats.acceleration += r32(2.5);
            }
            UpgradeEffect::Difficulty => {
                self.difficulty_raw += self.config.difficulty.upgrade_amount;
                self.score_multiplier += self.config.score.upgrade_multiplier;
            }
            UpgradeEffect::Weapon(weapon) => {
                self.player.active_weapon = weapon;
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
        self.objects.retain(|object| {
            let alive = !object.dead
                && self
                    .rooms
                    .iter()
                    .any(|(_, room)| room.area.contains(object.collider.position));
            if !alive {
                match &object.kind {
                    &ObjectKind::ExplosiveBarrel { range, damage } => {
                        let explosion =
                            Collider::new(object.collider.position, Shape::circle(range));
                        for enemy in &mut self.enemies {
                            if enemy.invincibility.is_min() && explosion.check(&enemy.body.collider)
                            {
                                enemy.health.change(-damage);
                            }
                        }

                        self.particles_queue.push(SpawnParticles {
                            kind: ParticleKind::Damage,
                            distribution: ParticleDistribution::Circle {
                                center: object.collider.position,
                                radius: range,
                            },
                            ..default()
                        });
                        self.events.push(Event::Sound(SoundEvent::Explosion));
                    }
                }
            }
            alive
        });

        self.minions.retain(|minion| {
            let alive = minion.health.is_above_min();
            if !alive {
                match minion.ai {
                    MinionAI::Bullet {
                        explosion_damage,
                        explosion_radius,
                        ..
                    } => {
                        let explosion = Collider::new(
                            minion.body.collider.position,
                            Shape::circle(explosion_radius),
                        );
                        for enemy in &mut self.enemies {
                            if enemy.invincibility.is_min() && explosion.check(&enemy.body.collider)
                            {
                                enemy.health.change(-explosion_damage);
                            }
                        }

                        self.particles_queue.push(SpawnParticles {
                            kind: ParticleKind::Damage,
                            distribution: ParticleDistribution::Circle {
                                center: minion.body.collider.position,
                                radius: explosion_radius,
                            },
                            ..default()
                        });
                    }
                }
            }
            alive
        });

        let in_battle = !self.enemies.is_empty() || !self.spawn_queue.is_empty();
        self.enemies.retain(|enemy| {
            let alive = enemy.health.is_above_min();
            if !alive {
                if enemy.is_boss {
                    self.bosses_killed += 1;
                }
                if self.player.health.is_above_min() {
                    self.score += (enemy.stats.score.unwrap_or(0) as f32
                        * self.score_multiplier.as_f32())
                        as Score;
                }
                if let EnemyAI::Bullet = enemy.ai {
                    self.particles_queue.push(SpawnParticles {
                        kind: ParticleKind::Bounce,
                        distribution: ParticleDistribution::Circle {
                            center: enemy.body.collider.position,
                            radius: r32(0.3),
                        },
                        ..default()
                    });
                } else {
                    self.events.push(Event::Sound(SoundEvent::Kill));
                }
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
        let mut options = vec![
            UpgradeEffect::Width,
            UpgradeEffect::Range,
            UpgradeEffect::Damage,
            UpgradeEffect::Speed,
            UpgradeEffect::Difficulty,
        ];
        options.extend(
            [Weapon::Whip, Weapon::Dash, Weapon::Bow, Weapon::FishingRod]
                .into_iter()
                .filter(|weapon| *weapon != self.player.active_weapon)
                .map(UpgradeEffect::Weapon),
        );
        let options: Vec<_> = options
            .choose_multiple(&mut rng, self.config.upgrades_per_level)
            .collect();
        let upgrades = options.iter().enumerate().map(|(i, effect)| Upgrade {
            collider: Collider::new(
                room.area.center() + offset * r32(i as f32 - (options.len() as f32 - 1.0) / 2.0),
                Shape::circle(0.5),
            ),
            effect: (*effect).clone(),
        });
        self.upgrades.extend(upgrades);

        self.rooms_cleared += 1;
        self.difficulty_raw += self.config.difficulty.room_bonus
            * self
                .config
                .difficulty
                .room_exponent
                .powf(r32(self.rooms_cleared as f32));
        self.score +=
            (self.config.score.room_bonus as f32 * self.score_multiplier.as_f32()) as Score;
    }

    pub fn enemy_ai(&mut self, delta_time: Time) {
        let mut rng = thread_rng();

        let ids: Vec<_> = self.enemies.ids().copied().collect();
        for id in ids {
            // NOTE: remove from the collection and insert back later
            // to allow processing other enemies at the same time
            let Some(mut enemy) = self.enemies.remove(&id) else {
                log::error!("enemy killed before it could control itself");
                continue;
            };

            enemy.invincibility.change(-delta_time);
            let repel_force = self
                .enemies
                .iter()
                .map(|other| (other.body.collider.position, 3.0, 1.0))
                .chain(
                    self.objects
                        .iter()
                        .map(|object| (object.collider.position, 1.5, 5.0)),
                )
                .map(|(other, pow, weight)| {
                    let delta = enemy.body.collider.position - other;
                    let len = delta.len();
                    if len.approx_eq(&Coord::ZERO) {
                        vec2::ZERO
                    } else {
                        delta / len.powf(r32(pow)) * r32(weight)
                    }
                })
                .fold(vec2::ZERO, vec2::add);

            match &mut enemy.ai {
                EnemyAI::Idle => {
                    let drag = r32(0.9);
                    enemy.body.velocity *= drag;
                    enemy.body.move_rotation();
                }
                EnemyAI::Bullet => {
                    enemy.body.move_rotation();
                }
                EnemyAI::Crawler => {
                    let target = self.player.body.collider.position;
                    let target_velocity = (target - enemy.body.collider.position + repel_force)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                    enemy.body.move_rotation();
                }
                EnemyAI::Shooter {
                    preferred_distance,
                    charge,
                    bullet,
                } => {
                    charge.change(delta_time);
                    if charge.is_max() {
                        charge.set_ratio(Time::ZERO);
                        let mut bullet = Enemy::new(
                            self.id_gen.gen(),
                            EnemyConfig {
                                health: bullet.health
                                    + self.config.difficulty.enemy_health_scaling * self.difficulty,
                                ..(**bullet).clone()
                            },
                            enemy.body.collider.position,
                        );
                        let dir = (self.player.body.collider.position
                            - bullet.body.collider.position)
                            .normalize_or_zero();
                        bullet.body.velocity = dir * bullet.stats.speed;
                        self.spawn_queue.push(bullet);
                    }

                    let target = self.player.body.collider.position
                        + (enemy.body.collider.position - self.player.body.collider.position)
                            .normalize_or_zero()
                            * *preferred_distance;
                    let target_velocity = (target - enemy.body.collider.position + repel_force)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                    enemy.body.move_rotation();
                }
                EnemyAI::Healer {
                    range,
                    heal_ratio,
                    cooldown,
                } => {
                    self.particles_queue.push(SpawnParticles {
                        kind: ParticleKind::Heal,
                        distribution: ParticleDistribution::Circle {
                            center: enemy.body.collider.position,
                            radius: r32(0.2),
                        },
                        density: r32(30.0) * delta_time,
                        ..default()
                    });

                    cooldown.change(-delta_time);
                    let mut heal_target = None;
                    // Heal closest enemy in range
                    if let Some((distance, target)) = self
                        .enemies
                        .iter_mut()
                        .map(|target| {
                            (
                                (enemy.body.collider.position - target.body.collider.position)
                                    .len(),
                                target,
                            )
                        })
                        .filter(|(_, enemy)| !enemy.health.is_max())
                        .min_by_key(|(d, _)| *d)
                    {
                        heal_target = Some(target.body.collider.position);
                        if cooldown.is_min() && distance < *range {
                            cooldown.set_ratio(Time::ONE);
                            target.health.change(target.health.max() * *heal_ratio);
                            self.particles_queue.push(SpawnParticles {
                                kind: ParticleKind::Heal,
                                distribution: ParticleDistribution::Circle {
                                    center: target.body.collider.position,
                                    radius: r32(1.2),
                                },
                                ..default()
                            });
                        }
                    }

                    let target = heal_target.map_or_else(
                        || {
                            self.player.body.collider.position
                                + (enemy.body.collider.position
                                    - self.player.body.collider.position)
                                    .normalize_or_zero()
                                    * *range
                                    * r32(1.5)
                        },
                        |target| {
                            target
                                + (enemy.body.collider.position - target).normalize_or_zero()
                                    * *range
                                    / r32(2.0)
                        },
                    );
                    let target_velocity = (target - enemy.body.collider.position + repel_force)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                    enemy.body.move_rotation();
                }
                EnemyAI::Shielder {
                    preferred_distance,
                    target,
                } => {
                    self.particles_queue.push(SpawnParticles {
                        kind: ParticleKind::Shield,
                        distribution: ParticleDistribution::Circle {
                            center: enemy.body.collider.position,
                            radius: r32(0.3),
                        },
                        density: r32(50.0) * delta_time,
                        ..default()
                    });

                    let target = match target {
                        Some(id) => {
                            let mut target = self.enemies.get_mut(id);
                            if let Some(target) = &mut target {
                                target.invincibility.set_ratio(Time::ONE);
                            }
                            target
                        }
                        None => {
                            let unit = self
                                .enemies
                                .iter_mut()
                                .filter(|target| !matches!(target.ai, EnemyAI::Shielder { .. }))
                                .min_by_key(|target| {
                                    (enemy.body.collider.position - target.body.collider.position)
                                        .len()
                                });
                            if let Some(unit) = &unit {
                                *target = Some(unit.id);
                            }
                            unit
                        }
                    };
                    let target = target.map(|target| target.body.collider.position);
                    let target = target.map_or_else(
                        || {
                            self.player.body.collider.position
                                + (enemy.body.collider.position
                                    - self.player.body.collider.position)
                                    .normalize_or_zero()
                                    * *preferred_distance
                                    * r32(1.5)
                        },
                        |target| {
                            target
                                + (enemy.body.collider.position - target).normalize_or_zero()
                                    * *preferred_distance
                        },
                    );
                    let target_velocity = (target - enemy.body.collider.position + repel_force)
                        .normalize_or_zero()
                        * enemy.stats.speed;
                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                        .clamp_len(..=enemy.stats.acceleration * delta_time);
                    enemy.body.move_rotation();
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
                                        timer: Bounded::new_max(r32(3.0)),
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
                                spawn_1up: Bounded::new(r32(1.0), r32(0.0)..=r32(5.0)),
                                target: None,
                            };
                        }
                    }
                },
                EnemyAI::Helicopter { helicopter } => {
                    self.events.push(Event::Sound(SoundEvent::Helicopter));
                    if let Some((_, room)) = self
                        .rooms
                        .iter()
                        .find(|(_, room)| room.area.contains(enemy.body.collider.position))
                    {
                        match &mut helicopter.state {
                            HelicopterState::Idle => {
                                // Oscilate
                                helicopter.oscilate.change(-delta_time);
                                if helicopter.oscilate.is_min() {
                                    helicopter.oscilate.set_ratio(Time::ONE);
                                    // Change target
                                    let points = room
                                        .area
                                        .extend_uniform(-r32(5.0))
                                        .corners()
                                        .into_iter()
                                        .filter(|pos| {
                                            (*pos - enemy.body.collider.position).len_sqr()
                                                > r32(1.0)
                                        });
                                    if let Some(point) = points.choose(&mut rng) {
                                        helicopter.state = HelicopterState::Moving(point);
                                    }
                                }
                            }
                            &mut HelicopterState::Moving(target) => {
                                let delta = target - enemy.body.collider.position;
                                if delta.len_sqr() < r32(1.0) {
                                    if rng.gen_bool(0.3) {
                                        // Minions
                                        let tank = &self.config.enemies["tank"];
                                        let circle = &self.config.enemies["circle"];
                                        let shielder = &self.config.enemies["shielder"];
                                        let shooter = &self.config.enemies["shooter"];
                                        let minions = [
                                            tank.clone(),
                                            shooter.clone(),
                                            circle.clone(),
                                            shielder.clone(),
                                            circle.clone(),
                                            circle.clone(),
                                            shooter.clone(),
                                        ]
                                        .into_iter()
                                        .map(|mut enemy| {
                                            enemy.health *= r32(1.5);
                                            enemy
                                        })
                                        .collect();
                                        helicopter.state = HelicopterState::Minions {
                                            minions,
                                            delay: Bounded::new_max(r32(0.15)),
                                        };
                                    } else {
                                        helicopter.state = HelicopterState::Minigun {
                                            timer: r32(5.0),
                                            shot_delay: Bounded::new_max(r32(0.2)),
                                        };
                                    }
                                } else {
                                    let target_velocity = delta.clamp_len(..=enemy.stats.speed);
                                    let acc = if vec2::dot(target_velocity, enemy.body.velocity)
                                        < Coord::ZERO
                                    {
                                        enemy.stats.acceleration * r32(2.0)
                                    } else {
                                        enemy.stats.acceleration
                                    };
                                    enemy.body.velocity += (target_velocity - enemy.body.velocity)
                                        .clamp_len(..=acc * delta_time);
                                }
                            }
                            HelicopterState::Minigun { timer, shot_delay } => {
                                // Shoot
                                shot_delay.change(-delta_time);
                                if shot_delay.is_min() {
                                    shot_delay.set_ratio(Time::ONE);

                                    let center = enemy.body.collider.position;
                                    let poss = [
                                        center + vec2(1.3, 0.0).as_r32(),
                                        center - vec2(1.3, 0.0).as_r32(),
                                    ];
                                    for pos in poss {
                                        let mut bullet = Enemy::new(
                                            self.id_gen.gen(),
                                            (*helicopter.minigun_bullet).clone(),
                                            pos,
                                        );
                                        let target = self.player.body.collider.position;
                                        let dir = (target - pos).normalize_or_zero();
                                        bullet.body.velocity = dir * bullet.stats.speed;
                                        self.spawn_queue.push(bullet);
                                        self.events.push(Event::Sound(SoundEvent::Minigun));
                                    }
                                }

                                // Update timer
                                *timer -= delta_time;
                                if *timer <= Time::ZERO {
                                    helicopter.state = HelicopterState::Idle;
                                }
                            }
                            HelicopterState::Minions { minions, delay } => {
                                delay.change(-delta_time);
                                if delay.is_min() {
                                    delay.set_ratio(Time::ONE);

                                    if let Some(minion) = minions.pop() {
                                        self.spawn_queue.push(Enemy::new(
                                            self.id_gen.gen(),
                                            EnemyConfig {
                                                health: minion.health
                                                    + self.config.difficulty.enemy_health_scaling
                                                        * self.difficulty,
                                                ..minion
                                            },
                                            enemy.body.collider.position,
                                        ));
                                    } else {
                                        helicopter.state = HelicopterState::Idle;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            enemy.body.collider.position += enemy.body.velocity * delta_time;
            enemy.body.collider.rotation += enemy.body.angular_velocity * delta_time;

            if let Some((id, offset)) = enemy.attached_to {
                match self.enemies.get_mut(&id) {
                    None => {
                        enemy.attached_to = None;
                    }
                    Some(other) => {
                        let vel = (enemy.body.velocity + other.body.velocity) / r32(2.0);
                        enemy.body.velocity = vel;
                        other.body.velocity = vel;

                        let delta = enemy.body.collider.position - other.body.collider.position;
                        let off = offset - delta;
                        enemy.body.collider.position += off / r32(2.0);
                        other.body.collider.position -= off / r32(2.0);
                        enemy.body.collider.rotation = delta.arg();
                        other.body.collider.rotation = (-delta).arg();
                    }
                }
            }

            self.enemies.insert(enemy);
        }
    }

    pub fn minion_ai(&mut self, delta_time: Time) {
        for minion in &mut self.minions {
            match &mut minion.ai {
                MinionAI::Bullet { .. } => {
                    minion.body.move_rotation();
                }
            }

            minion.body.collider.position += minion.body.velocity * delta_time;
            minion.body.collider.rotation += minion.body.angular_velocity * delta_time;
        }
    }

    pub fn damage_around(&mut self, drawing: Drawing, width: Coord, base_damage: Hp) {
        for enemy in &mut self.enemies {
            if enemy.invincibility.is_above_min() {
                continue;
            }

            let Some(delta) =
                delta_to_chain(enemy.body.collider.position, &drawing.points_smoothed)
            else {
                continue;
            };

            let enemy_radius = enemy.body.collider.compute_aabb().size().len()
                / r32(std::f32::consts::SQRT_2 * 2.0);
            if delta.len() < width + enemy_radius {
                enemy.health.change(-base_damage); // TODO: combo scaling
                enemy.last_hit = self.game_time;

                if let Weapon::FishingRod = &self.player.active_weapon {
                    if drawing.points_smoothed.len() >= 2 {
                        let speed = self.player.stats.fishing.speed;
                        let dir = (*drawing.points_smoothed.last().unwrap()
                            - drawing.points_smoothed[drawing.points_smoothed.len() - 2])
                            .normalize_or_zero();
                        enemy.body.velocity += dir * speed;
                    }
                }

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
        for object in &mut self.objects {
            let Some(delta) = delta_to_chain(object.collider.position, &drawing.points_smoothed)
            else {
                continue;
            };

            let object_radius =
                object.collider.compute_aabb().size().len() / r32(std::f32::consts::SQRT_2 * 2.0);
            if delta.len() < width + object_radius {
                object.dead = true;
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

        let find_position = |rng: &mut ThreadRng| -> Option<Position> {
            let area = room.area.extend_uniform(-r32(3.0));
            for _ in 0..50 {
                let position = vec2(
                    rng.gen_range(area.min.x..=area.max.x),
                    rng.gen_range(area.min.y..=area.max.y),
                );
                if (self.player.body.collider.position - position).len() > r32(5.0) {
                    return Some(position);
                }
            }
            None
        };
        let mut spawn_enemy = |config: &EnemyConfig, position: Position| -> Enemy {
            let id = self.id_gen.gen();
            Enemy::new(
                id,
                EnemyConfig {
                    health: config.health
                        + self.config.difficulty.enemy_health_scaling * self.difficulty,
                    ..config.clone()
                },
                position,
            )
        };

        if let Some(boss) = self
            .config
            .bosses
            .iter()
            .find(|boss| boss.room == self.rooms_cleared + 1)
        {
            // Boss room
            for enemy in &boss.enemies {
                let Some(enemy) = self.config.enemies.get(enemy) else {
                    log::error!("Enemy named {enemy:?} not found");
                    continue;
                };
                if let Some(position) = find_position(&mut rng) {
                    let mut boss = spawn_enemy(enemy, position);
                    boss.is_boss = true;
                    // Boost boss max hp with player damage
                    let hp_boost = self.player.stats.whip.damage / r32(7.0) * r32(0.9);
                    boss.health = Bounded::new_max(boss.health.max() * hp_boost);
                    self.enemies.insert(boss);
                }
            }
            return;
        }

        while let Some(config) = self
            .config
            .enemies
            .values()
            .filter(|config| config.cost.map_or(false, |cost| cost <= difficulty))
            .choose(&mut rng)
        {
            if let Some(grouping) = &config.grouping {
                if grouping.cost <= difficulty
                    && rng.gen_bool(grouping.chance.as_f32().clamp(0.0, 1.0).into())
                {
                    // Spawn group
                    difficulty -= grouping.cost;
                    if let Some(position) = find_position(&mut rng) {
                        match &config.shape {
                            Shape::Circle { radius } => {
                                // Hexagon
                                let mut prev = None;
                                for i in 0..6 {
                                    let angle = Angle::from_degrees(r32(360.0 * i as f32 / 6.0));
                                    let position = position + angle.unit_vec() * *radius * r32(2.0);
                                    let mut enemy = spawn_enemy(config, position);
                                    if let Some((prev, prev_pos)) = prev {
                                        enemy.attached_to =
                                            Some((prev, enemy.body.collider.position - prev_pos));
                                    }
                                    prev = Some((enemy.id, enemy.body.collider.position));
                                    self.enemies.insert(enemy);
                                }
                            }
                            Shape::Rectangle { .. } => {
                                // 3x3 square
                                let poss = [
                                    (-1, 1),
                                    (0, 1),
                                    (1, 1),
                                    (1, 0),
                                    (1, -1),
                                    (0, -1),
                                    (-1, -1),
                                    (-1, 0),
                                ];
                                let mut prev = None;
                                for (x, y) in poss {
                                    let position = position + vec2(x, y).as_r32();
                                    let mut enemy = spawn_enemy(config, position);
                                    if let Some((prev, prev_pos)) = prev {
                                        enemy.attached_to =
                                            Some((prev, enemy.body.collider.position - prev_pos));
                                    }
                                    prev = Some((enemy.id, enemy.body.collider.position));
                                    self.enemies.insert(enemy);
                                }
                            }
                            Shape::Triangle { .. } => {
                                // TODO
                            }
                        }
                    }
                    continue;
                }
            }

            difficulty -= config.cost.unwrap_or(R32::ZERO);
            if let Some(position) = find_position(&mut rng) {
                self.enemies.insert(spawn_enemy(config, position));
            }
        }

        // Objects
        let objects = [(
            ObjectKind::ExplosiveBarrel {
                range: self.player.stats.whip.width + r32(2.0),
                damage: self.player.stats.whip.damage * r32(1.7),
            },
            0.4,
        )];
        for (kind, chance) in objects {
            if !rng.gen_bool(chance) {
                continue;
            }

            let area = room.area.extend_uniform(-r32(3.0));
            for _ in 0..10 {
                let position = vec2(
                    rng.gen_range(area.min.x..=area.max.x),
                    rng.gen_range(area.min.y..=area.max.y),
                );
                if self
                    .enemies
                    .iter()
                    .all(|enemy| (enemy.body.collider.position - position).len() > r32(5.0))
                {
                    self.objects.push(Object {
                        dead: false,
                        collider: Collider::new(position, Shape::square(r32(1.33))),
                        kind,
                    });
                    break;
                }
            }
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

        let min = r32(1.0);
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

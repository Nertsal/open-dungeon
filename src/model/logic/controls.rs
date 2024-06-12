use super::*;

impl Model {
    pub fn controls(&mut self, input: PlayerControls, delta_time: Time) {
        let player = &mut self.player;

        // Movement
        if player.draw_action.is_some() {
            player.body.velocity = vec2::ZERO;
        } else {
            let move_dir = input.move_dir.clamp_len(..=Coord::ONE);
            let target_velocity = move_dir * player.stats.speed;
            player.body.velocity += (target_velocity - player.body.velocity)
                .clamp_len(..=player.stats.acceleration * delta_time);
        }
        player.body.collider.position += player.body.velocity * delta_time;
        player.body.collider.rotation += player.body.angular_velocity * delta_time;

        match input.drawing {
            Some(position) => {
                // Drawing
                if player.draw_action.is_none() {
                    player.draw_action = Some(Drawing {
                        points_raw: vec![DrawPoint {
                            position: player.body.collider.position,
                            time: self.real_time,
                        }],
                        points_smoothed: Vec::new(),
                    });
                }

                let mut point = DrawPoint {
                    position,
                    time: self.real_time,
                };

                let drawing = player.draw_action.as_mut().unwrap();
                let remaining = player.stats.dash.max_distance - drawing.length();
                if remaining > Coord::ZERO {
                    // Add a point
                    let last = drawing
                        .points_raw
                        .last()
                        .expect("drawing has to have at least one starting point");
                    point.position =
                        last.position + (point.position - last.position).clamp_len(..=remaining);
                    drawing.points_raw.push(point);

                    // Update smooth
                    let points = drawing
                        .points_raw
                        .iter()
                        .map(|point| point.position.as_f32())
                        .dedup_by(|a, b| (*a - *b).len_sqr() < 0.01)
                        .collect();
                    let chain = CardinalSpline::new(points, 0.5).chain(3);
                    drawing.points_smoothed =
                        chain.vertices.into_iter().map(|pos| pos.as_r32()).collect();
                }

                self.particles_queue.push(SpawnParticles {
                    density: r32(0.5),
                    kind: ParticleKind::Draw,
                    distribution: ParticleDistribution::Drawing {
                        points: drawing.points_smoothed.clone(),
                        width: r32(0.2),
                    },
                    ..default()
                });
            }
            None => self.stop_drawing(),
        }
    }

    pub fn stop_drawing(&mut self) {
        let Some(drawing) = self.player.draw_action.take() else {
            return;
        };

        self.player_dash(drawing);
    }

    pub fn player_dash(&mut self, drawing: Drawing) {
        if drawing.points_smoothed.len() < 2 {
            return;
        }

        let room = self
            .rooms
            .iter()
            .find(|(_, room)| room.area.contains(self.player.body.collider.position))
            .map(|(idx, _)| idx);

        let &last = drawing.points_smoothed.last().unwrap();
        let &prelast = drawing
            .points_smoothed
            .get(drawing.points_smoothed.len() - 2)
            .unwrap();

        self.player.body.collider.position = last;
        self.player.body.velocity =
            (last - prelast).normalize_or_zero() * self.config.player.dash.speed;

        self.damage_around(
            drawing,
            self.player.stats.dash.width,
            self.player.stats.dash.damage,
        );

        if let Some(room) = room {
            self.unlock_room(room, self.player.body.collider.position);
        }
    }
}

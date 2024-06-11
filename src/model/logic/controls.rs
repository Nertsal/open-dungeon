use super::*;

impl Model {
    pub fn controls(&mut self, input: PlayerControls, delta_time: Time) {
        let player = &mut self.player;

        // Movement
        let move_dir = input.move_dir.clamp_len(..=Coord::ONE);
        let target_velocity = move_dir * player.stats.speed;
        player.body.velocity += (target_velocity - player.body.velocity)
            .clamp_len(..=player.stats.acceleration * delta_time);
        player.body.collider.position += player.body.velocity * delta_time;
        player.body.collider.rotation += player.body.angular_velocity * delta_time;

        match input.drawing {
            Some(position) => {
                // Drawing
                let point = DrawPoint {
                    position,
                    time: self.real_time,
                };
                match &mut player.draw_action {
                    Some(drawing) => {
                        if drawing.length() < player.stats.dash.max_distance {
                            drawing.points_raw.push(point);
                        }
                    }
                    action @ None => {
                        *action = Some(Drawing {
                            points_raw: vec![point],
                            points_smoothed: Vec::new(),
                        })
                    }
                }

                let drawing = player
                    .draw_action
                    .as_mut()
                    .expect("draw action must be set");
                let points = drawing
                    .points_raw
                    .iter()
                    .map(|point| point.position.as_f32())
                    .dedup_by(|a, b| (*a - *b).len_sqr() < 0.01)
                    .collect();
                let chain = CardinalSpline::new(points, 0.5).chain(3);
                drawing.points_smoothed =
                    chain.vertices.into_iter().map(|pos| pos.as_r32()).collect();

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
    }
}

use super::*;

impl Model {
    pub fn controls(&mut self, input: PlayerControls, delta_time: Time) {
        let player = &mut self.player;

        // Movement
        let move_dir = input.move_dir.clamp_len(..=Coord::ONE);
        let target_velocity = move_dir * player.stats.speed;
        player.velocity += (target_velocity - player.velocity)
            .clamp_len(..=player.stats.acceleration * delta_time);
        player.collider.position += player.velocity * delta_time;

        match input.drawing {
            Some(position) => {
                // Drawing
                let point = DrawPoint {
                    position,
                    time: self.real_time,
                };
                match &mut player.draw_action {
                    Some(drawing) => drawing.points_raw.push_back(point),
                    action @ None => {
                        *action = Some(Drawing {
                            points_raw: vec![point].into(),
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

        self.player.collider.position = last;
        self.player.velocity = (last - prelast).normalize_or_zero() * self.config.player.dash.speed;

        self.damage_around(
            drawing,
            self.player.stats.dash.width,
            self.player.stats.dash.damage,
        );
    }
}

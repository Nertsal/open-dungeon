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
                    Some(drawing) => drawing.points.push_back(point),
                    action @ None => {
                        *action = Some(Drawing {
                            points: vec![point].into(),
                        })
                    }
                }
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
        if drawing.points.len() < 2 {
            return;
        }

        let last = drawing.points.back().unwrap();
        let prelast = drawing.points.get(drawing.points.len() - 2).unwrap();

        self.player.collider.position = last.position;
        self.player.velocity =
            (last.position - prelast.position).normalize_or_zero() * self.config.player.dash_speed;
    }
}

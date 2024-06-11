use super::*;

impl Model {
    pub fn update(&mut self, input: PlayerControls, delta_time: Time) {
        self.real_time += delta_time;
        self.game_time += delta_time;

        self.controls(input, delta_time);
        self.collisions(delta_time);
    }

    pub fn controls(&mut self, input: PlayerControls, delta_time: Time) {
        let player = &mut self.player;

        // Movement
        let move_dir = input.move_dir.clamp_len(..=Coord::ONE);
        player.collider.position += move_dir * player.speed * delta_time;

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
    }

    pub fn collisions(&mut self, _delta_time: Time) {
        // Player - Object collisions
        let player = &mut self.player;
        for object in &self.objects {
            if let Some(collision) = player.collider.collide(&object.collider) {
                player.collider.position -= collision.normal * collision.penetration;
            }
        }
    }
}

use super::*;

impl Model {
    pub fn update(&mut self, player_input_dir: vec2<Coord>, delta_time: Time) {
        self.controls(player_input_dir, delta_time);
        self.collisions(delta_time);
    }

    pub fn controls(&mut self, player_input_dir: vec2<Coord>, delta_time: Time) {
        let player_input_dir = player_input_dir.clamp_len(..=Coord::ONE);
        self.player.collider.position += player_input_dir * self.player.speed * delta_time;
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

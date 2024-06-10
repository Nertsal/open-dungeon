use super::*;

impl Model {
    pub fn update(&mut self, player_input_dir: vec2<Coord>, delta_time: Time) {
        self.controls(player_input_dir, delta_time);
    }

    pub fn controls(&mut self, player_input_dir: vec2<Coord>, delta_time: Time) {
        let player_input_dir = player_input_dir.clamp_len(..=Coord::ONE);
        self.player.collider.position += player_input_dir * self.player.speed * delta_time;
    }
}

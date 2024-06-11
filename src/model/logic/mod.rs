mod controls;

use super::*;

impl Model {
    pub fn update(&mut self, input: PlayerControls, delta_time: Time) {
        self.real_time += delta_time;
        self.game_time += delta_time;

        self.controls(input, delta_time);
        self.collisions(delta_time);
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

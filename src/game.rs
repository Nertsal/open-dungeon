use crate::{prelude::*, render::GameRender};

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,

    render: GameRender,
    model: Model,
}

impl GameState {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),

            render: GameRender::new(geng, assets),
            model: Model::new(assets.config.clone()),
        }
    }
}

impl geng::State for GameState {
    fn fixed_update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);

        let window = self.geng.window();

        let mut move_dir = vec2::<f32>::ZERO;
        if geng_utils::key::is_key_pressed(window, &self.assets.controls.left) {
            move_dir.x -= 1.0;
        }
        if geng_utils::key::is_key_pressed(window, &self.assets.controls.right) {
            move_dir.x += 1.0;
        }
        if geng_utils::key::is_key_pressed(window, &self.assets.controls.down) {
            move_dir.y -= 1.0;
        }
        if geng_utils::key::is_key_pressed(window, &self.assets.controls.up) {
            move_dir.y += 1.0;
        }

        self.model.update(move_dir.as_r32(), delta_time);
    }

    fn draw(&mut self, framebuffer: &mut geng::prelude::ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        self.render.draw_game(&self.model, framebuffer);
    }
}

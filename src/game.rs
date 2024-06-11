use crate::{prelude::*, render::GameRender};

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,

    framebuffer_size: vec2<usize>,
    cursor: CursorState,
    render: GameRender,
    model: Model,
}

#[derive(Debug, Clone)]
pub struct CursorState {
    pub screen_pos: vec2<f64>,
    pub world_pos: Position,
}

impl GameState {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),

            framebuffer_size: vec2(1, 1),
            cursor: CursorState {
                screen_pos: vec2::ZERO,
                world_pos: vec2::ZERO,
            },
            render: GameRender::new(geng, assets),
            model: Model::new(assets.config.clone()),
        }
    }

    fn get_controls(&self) -> PlayerControls {
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

        let drawing = geng_utils::key::is_key_pressed(window, &self.assets.controls.draw)
            .then_some(self.cursor.world_pos);

        PlayerControls {
            move_dir: move_dir.as_r32(),
            drawing,
        }
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        let _delta_time = Time::new(delta_time as f32);
        self.cursor.world_pos = self
            .model
            .camera
            .screen_to_world(
                self.framebuffer_size.as_f32(),
                self.cursor.screen_pos.as_f32(),
            )
            .as_r32();
    }

    fn fixed_update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);

        let input = self.get_controls();
        self.model.update(input, delta_time);
    }

    fn handle_event(&mut self, event: geng::Event) {
        if let geng::Event::CursorMove { position } = event {
            self.cursor.screen_pos = position;
        }
    }

    fn draw(&mut self, framebuffer: &mut geng::prelude::ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);

        self.render.draw_game(&self.model, framebuffer);
    }
}

use crate::{
    prelude::*,
    render::{GameRender, SwapBuffer},
};

pub struct GameState {
    geng: Geng,
    assets: Rc<Assets>,

    framebuffer_size: vec2<usize>,
    cursor: CursorState,
    pixel_buffer: SwapBuffer,
    post_buffer: SwapBuffer,
    unit_quad: ugli::VertexBuffer<draw2d::TexturedVertex>,

    render: GameRender,
    model: Model,

    drawing_sfx: geng::SoundEffect,
    helicopter_sfx: geng::SoundEffect,
    volume: f32,
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
            pixel_buffer: SwapBuffer::new(geng.ugli(), crate::GAME_RESOLUTION),
            post_buffer: SwapBuffer::new(geng.ugli(), vec2(1, 1)),
            unit_quad: geng_utils::geometry::unit_quad_geometry(geng.ugli()),

            render: GameRender::new(geng, assets),
            model: Model::new(assets.config.clone()),

            drawing_sfx: {
                let mut sfx = assets.sounds.drawing.play();
                sfx.set_volume(0.0);
                sfx
            },
            helicopter_sfx: {
                let mut sfx = assets.sounds.helicopter.play();
                sfx.set_volume(0.0);
                sfx
            },
            volume: 0.5,
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

    fn play_sfx(&self, sfx: &geng::Sound) {
        let mut sfx = sfx.play();
        sfx.set_volume(self.volume);
    }
}

impl geng::State for GameState {
    fn update(&mut self, delta_time: f64) {
        let _delta_time = Time::new(delta_time as f32);

        let game_pos = geng_utils::layout::fit_aabb(
            self.pixel_buffer.size().as_f32(),
            Aabb2::ZERO.extend_positive(self.framebuffer_size.as_f32()),
            vec2(0.5, 0.5),
        );
        let pos = self.cursor.screen_pos.as_f32() - game_pos.bottom_left();
        self.cursor.world_pos = self
            .model
            .camera
            .screen_to_world(game_pos.size(), pos)
            .as_r32();
        self.model.cursor_pos = self.cursor.world_pos;
    }

    fn fixed_update(&mut self, delta_time: f64) {
        let delta_time = Time::new(delta_time as f32);

        let input = self.get_controls();
        self.model.update(input, delta_time);

        let mut drawing = false;
        let mut helicopter = false;
        let mut hit = false;
        let mut kill = false;
        for event in std::mem::take(&mut self.model.events) {
            match event {
                Event::Sound(sfx) => match sfx {
                    SoundEvent::Drawing => drawing = true,
                    SoundEvent::Helicopter => helicopter = true,
                    SoundEvent::Hit => hit = true,
                    SoundEvent::Kill => kill = true,
                    SoundEvent::HitSelf => self.play_sfx(&self.assets.sounds.hit_self),
                    SoundEvent::Bounce => self.play_sfx(&self.assets.sounds.bounce),
                    SoundEvent::Expand => self.play_sfx(&self.assets.sounds.expand),
                    SoundEvent::Minigun => self.play_sfx(&self.assets.sounds.minigun),
                },
            }
        }
        if kill {
            self.play_sfx(&self.assets.sounds.kill);
        } else if hit {
            self.play_sfx(&self.assets.sounds.hit);
        }
        self.drawing_sfx
            .set_volume(if drawing { 1.0 } else { 0.0 } * self.volume);
        self.helicopter_sfx
            .set_volume(if helicopter { 1.0 } else { 0.0 } * self.volume);
    }

    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyPress { key: geng::Key::R }
                if self.geng.window().is_key_pressed(geng::Key::ControlLeft) =>
            {
                self.model.reset();
            }
            geng::Event::CursorMove { position } => {
                self.cursor.screen_pos = position;
            }
            _ => (),
        }
    }

    fn draw(&mut self, framebuffer: &mut geng::prelude::ugli::Framebuffer) {
        self.post_buffer.update_size(framebuffer.size());
        self.framebuffer_size = framebuffer.size();
        ugli::clear(
            framebuffer,
            Some(self.assets.palette.background),
            None,
            None,
        );

        // Pixelated
        let pixel_buffer = &mut self.pixel_buffer.active_draw();
        ugli::clear(
            pixel_buffer,
            Some(self.assets.palette.background),
            None,
            None,
        );

        // Game
        self.render.draw_game(&self.model, pixel_buffer);

        // Background
        self.pixel_buffer.swap();
        let pixel_buffer = &mut geng_utils::texture::attach_texture(
            &mut self.pixel_buffer.active,
            self.geng.ugli(),
        );
        let world_matrix = (self
            .model
            .camera
            .projection_matrix(pixel_buffer.size().as_f32())
            * self.model.camera.view_matrix())
        .inverse();
        ugli::draw(
            pixel_buffer,
            &self.assets.shaders.background,
            ugli::DrawMode::TriangleFan,
            &self.unit_quad,
            ugli::uniforms! {
                u_texture: &self.pixel_buffer.second,
                u_time: self.model.real_time.as_f32(),
                u_mask_color: self.assets.palette.room,
                u_world_matrix: world_matrix,
            },
            ugli::DrawParameters {
                blend_mode: Some(ugli::BlendMode::straight_alpha()),
                ..default()
            },
        );

        // Upscale
        let post_buffer = &mut self.post_buffer.active_draw();
        geng_utils::texture::DrawTexture::new(&self.pixel_buffer.active)
            .fit_screen(vec2(0.5, 0.5), post_buffer)
            .draw(&geng::PixelPerfectCamera, &self.geng, post_buffer);

        // UI
        self.render.draw_ui(&self.model, post_buffer);

        // Postprocessing - Hurt
        self.post_buffer.swap();
        let post_buffer = &mut geng_utils::texture::attach_texture(
            &mut self.post_buffer.active,
            self.geng.ugli(),
        );
        let intensity = 1.0
            - (self.model.game_time - self.model.player.last_hit)
                .as_f32()
                .min(1.0);
        ugli::draw(
            post_buffer,
            &self.assets.shaders.vhs,
            ugli::DrawMode::TriangleFan,
            &self.unit_quad,
            ugli::uniforms! {
                u_texture: &self.post_buffer.second,
                u_time: self.model.real_time.as_f32(),
                u_intensity: intensity,
            },
            ugli::DrawParameters { ..default() },
        );

        geng_utils::texture::DrawTexture::new(&self.post_buffer.active)
            .fit_screen(vec2(0.5, 0.5), framebuffer)
            .draw(&geng::PixelPerfectCamera, &self.geng, framebuffer);
    }
}

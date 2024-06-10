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
            model: Model::new(),
        }
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut geng::prelude::ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
    }
}

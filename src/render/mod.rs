use crate::prelude::*;

pub struct GameRender {
    geng: Geng,
    assets: Rc<Assets>,
}

impl GameRender {
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            geng: geng.clone(),
            assets: assets.clone(),
        }
    }

    pub fn draw_game(&mut self, model: &Model, framebuffer: &mut ugli::Framebuffer) {
        if let Some(drawing) = &model.player.draw_action {
            // Drawing
            let points = drawing
                .points
                .iter()
                .map(|point| point.position.as_f32())
                .dedup_by(|a, b| (*a - *b).len_sqr() < 0.01)
                .collect();
            // let chain = Chain::new(points);
            let chain = CardinalSpline::new(points, 0.5).chain(3);
            let chain = draw2d::Chain::new(chain, 0.1, Rgba::WHITE, 3);
            self.geng
                .draw2d()
                .draw2d(framebuffer, &model.camera, &chain);
        }

        // Objects
        for object in &model.objects {
            self.draw_collider(&object.collider, Rgba::RED, &model.camera, framebuffer);
        }

        // Player
        self.draw_collider(
            &model.player.collider,
            Rgba::GREEN,
            &model.camera,
            framebuffer,
        );
    }

    pub fn draw_collider(
        &self,
        collider: &Collider,
        color: Rgba<f32>,
        camera: &Camera,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let transform = collider.transform_mat().as_f32();
        match &collider.shape {
            Shape::Circle { radius } => {
                self.geng.draw2d().draw2d_transformed(
                    framebuffer,
                    camera,
                    &draw2d::Ellipse::circle(vec2::ZERO, radius.as_f32(), color),
                    transform,
                );
            }
            &Shape::Rectangle { width, height } => {
                let quad = Aabb2::ZERO.extend_symmetric(vec2(width, height).as_f32() / 2.0);
                self.geng.draw2d().draw2d_transformed(
                    framebuffer,
                    camera,
                    &draw2d::Quad::new(quad, color),
                    transform,
                );
            }
        }
    }
}

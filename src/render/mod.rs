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
        // Rooms
        for (_, room) in &model.rooms {
            self.geng.draw2d().quad(
                framebuffer,
                &model.camera,
                room.area.map(Coord::as_f32),
                self.assets.palette.room,
            );
        }

        // Objects
        for object in &model.objects {
            self.draw_collider(
                &object.collider,
                self.assets.palette.object,
                &model.camera,
                framebuffer,
            );
        }

        if let Some(drawing) = &model.player.draw_action {
            // Drawing
            let points = drawing
                .points_smoothed
                .iter()
                .map(|pos| pos.as_f32())
                .collect();
            let chain = Chain::new(points);
            let chain = draw2d::Chain::new(chain, 0.1, self.assets.palette.drawing, 3);
            self.geng
                .draw2d()
                .draw2d(framebuffer, &model.camera, &chain);
        }

        // Minions
        for minion in &model.minions {
            self.draw_collider(
                &minion.body.collider,
                self.assets.palette.minion,
                &model.camera,
                framebuffer,
            );
        }

        // Enemies
        for enemy in &model.enemies {
            let hit_t = ((model.game_time - enemy.last_hit).as_f32() / 0.5).min(1.0);
            let hit_t = (1.0 - hit_t) * 2.5 + 1.0;
            let hit_color = Rgba::<f32>::WHITE.map_rgb(|x| x * hit_t);
            let color = self.assets.palette.enemy * hit_color;

            match &enemy.ai {
                EnemyAI::Pacman { pacman } => {
                    let radius = enemy.body.collider.compute_aabb().size().len().as_f32()
                        / std::f32::consts::SQRT_2
                        / 2.0;
                    let rotation = enemy.body.collider.rotation.map(Coord::as_f32);
                    let resolution = 20;

                    let mut open_angle = Angle::from_degrees(15.0);
                    if let PacmanState::Power { .. } = &pacman.state {
                        open_angle += Angle::from_degrees(
                            15.0 * ((model.game_time.as_f32() * 3.0).fract() * 2.0 - 1.0),
                        );
                    }

                    let max_angle = Angle::from_degrees(360.0) - open_angle;
                    let vertices: Vec<_> = std::iter::once(-vec2(radius * 0.2, 0.0))
                        .chain((0..resolution).map(|i| {
                            let t = i as f32 / (resolution - 1) as f32;
                            let angle = open_angle + (max_angle - open_angle) * t;
                            angle.unit_vec() * radius
                        }))
                        .map(|pos| enemy.body.collider.position.as_f32() + pos.rotate(rotation))
                        .collect();
                    self.geng.draw2d().draw(
                        framebuffer,
                        &model.camera,
                        &vertices,
                        color,
                        ugli::DrawMode::TriangleFan,
                    );
                }
                EnemyAI::Helicopter { .. } => {
                    let radius = enemy.body.collider.compute_aabb().size().len().as_f32()
                        / std::f32::consts::SQRT_2
                        / 2.0;
                    let center = enemy.body.collider.position.as_f32();

                    // Body
                    self.geng.draw2d().circle_with_cut(
                        framebuffer,
                        &model.camera,
                        center,
                        radius - 0.1,
                        radius,
                        color,
                    );

                    let direction = (model.player.body.collider.position
                        - enemy.body.collider.position)
                        .x
                        .signum()
                        .as_f32();

                    // Tail
                    let a = center - vec2(radius, 0.0) * direction;
                    let b = center - vec2(radius * 2.5, 0.0) * direction;
                    self.geng.draw2d().draw2d(
                        framebuffer,
                        &model.camera,
                        &draw2d::Segment::new(Segment(a, b), radius * 0.1, color),
                    );
                    let a = b + vec2(0.0, radius * 0.3);
                    let b = b - vec2(0.0, radius * 0.3);
                    self.geng.draw2d().draw2d(
                        framebuffer,
                        &model.camera,
                        &draw2d::Segment::new(Segment(a, b), radius * 0.1, color),
                    );

                    // Blades
                    let blades = 2;
                    for i in 0..blades {
                        let t = i as f32 / blades as f32;
                        let angle = Angle::from_degrees(
                            t * 180.0 + (model.game_time.as_f32() * 2.0).fract() * 360.0,
                        );
                        let (sin, cos) = angle.as_radians().sin_cos();
                        let a = vec2(cos, sin) * vec2(1.2, 0.6) * radius;
                        let b = center + vec2(0.0, radius) - a;
                        let a = center + vec2(0.0, radius) + a;
                        self.geng.draw2d().draw2d(
                            framebuffer,
                            &model.camera,
                            &draw2d::Segment::new(Segment(a, b), radius * 0.15, color),
                        );
                    }

                    // Skids
                    let width = radius * 2.2;
                    let offset = -direction * vec2(radius * 0.2, 0.0);
                    let a = center + offset + vec2(-width / 2.0, -radius);
                    let b = center + offset + vec2(width / 2.0, -radius);
                    self.geng.draw2d().draw2d(
                        framebuffer,
                        &model.camera,
                        &draw2d::Segment::new(Segment(a, b), radius * 0.15, color),
                    );
                }
                _ => {
                    self.draw_outline(&enemy.body.collider, 0.1, color, &model.camera, framebuffer);
                }
            }
            if enemy.invincibility.is_above_min() {
                let shield = Collider::new(
                    enemy.body.collider.position,
                    Shape::circle(
                        enemy.body.collider.compute_aabb().size().len()
                            / r32(std::f32::consts::SQRT_2 * 2.0)
                            + r32(0.15),
                    ),
                );
                self.draw_outline(
                    &shield,
                    0.1,
                    self.assets.palette.drawing,
                    &model.camera,
                    framebuffer,
                );
            }
            self.draw_health_bar(
                &enemy.body.collider,
                &enemy.health,
                &model.camera,
                framebuffer,
            );
        }

        // 1up
        for up in &model.pacman_1ups {
            self.draw_collider(
                &up.collider,
                self.assets.palette.pacman_1up,
                &model.camera,
                framebuffer,
            );
        }

        // Upgrades
        for upgrade in &model.upgrades {
            // self.draw_collider(
            //     &upgrade.collider,
            //     self.assets.palette.upgrade,
            //     &model.camera,
            //     framebuffer,
            // );
            let texture = match &upgrade.effect {
                UpgradeEffect::Width => &self.assets.sprites.width,
                UpgradeEffect::Range => &self.assets.sprites.range,
                UpgradeEffect::Damage => &self.assets.sprites.damage,
                UpgradeEffect::Speed => &self.assets.sprites.speed,
                UpgradeEffect::Difficulty => &self.assets.sprites.skull,
                UpgradeEffect::Weapon(weapon) => match weapon {
                    Weapon::Whip => &self.assets.sprites.whip,
                    Weapon::Dash => &self.assets.sprites.dash,
                    Weapon::Bow => &self.assets.sprites.bow,
                    Weapon::FishingRod => &self.assets.sprites.fishing_rod,
                },
            };

            let pos = upgrade.collider.compute_aabb().center().as_f32();
            if let Some(pos) = model
                .camera
                .world_to_screen(framebuffer.size().as_f32(), pos)
            {
                let quad = Aabb2::point(pos);
                self.draw_texture(quad, texture, self.assets.palette.upgrade, framebuffer);
            }
        }

        // Player
        if model.player.invincibility.is_above_min() {
            let shield = Collider::new(
                model.player.body.collider.position,
                model.player.stats.shield,
            );
            self.draw_outline(
                &shield,
                0.1,
                self.assets.palette.enemy,
                &model.camera,
                framebuffer,
            );
        }
        self.draw_collider(
            &model.player.body.collider,
            self.assets.palette.player,
            &model.camera,
            framebuffer,
        );
        self.draw_health_bar(
            &model.player.body.collider,
            &model.player.health,
            &model.camera,
            framebuffer,
        );

        // Particles TODO instance
        for (_, particle) in &model.particles {
            let t = crate::util::smoothstep(particle.lifetime.get_ratio()).as_f32();
            let transform = mat3::scale_uniform(t);
            let mut color = match particle.kind {
                ParticleKind::Draw => self.assets.palette.dash,
                ParticleKind::Drawing => self.assets.palette.drawing,
                ParticleKind::WallBreakable => self.assets.palette.wall,
                ParticleKind::WallBlock => self.assets.palette.wall_block,
                ParticleKind::Bounce => self.assets.palette.collision,
                ParticleKind::Damage => self.assets.palette.damage,
                ParticleKind::Upgrade => self.assets.palette.upgrade,
                ParticleKind::HitSelf => self.assets.palette.player,
                ParticleKind::Shield => self.assets.palette.enemy,
                ParticleKind::Heal => self.assets.palette.idk,
            };
            color.a = t;
            self.draw_collider_transformed(
                transform,
                &particle.collider,
                color,
                &model.camera,
                framebuffer,
            );
        }

        // // Remaining dash charge
        // if let Some(drawing) = &model.player.draw_action {
        //     let ratio = (drawing.length() / model.player.stats.dash.max_distance).as_f32();
        //     if ratio < 1.0 {
        //         let draw =
        //             Aabb2::point(model.camera.center + vec2(0.0, model.camera.fov * 0.9 / 2.0))
        //                 .extend_symmetric(vec2(3.0, 0.3) / 2.0);
        //         let draw = draw.extend_symmetric(vec2(draw.width() * (-ratio), 0.0) / 2.0);
        //         self.geng.draw2d().quad(
        //             framebuffer,
        //             &model.camera,
        //             draw,
        //             self.assets.palette.drawing,
        //         );
        //     }
        // }
    }

    pub fn draw_texture(
        &self,
        quad: Aabb2<f32>,
        texture: &ugli::Texture,
        color: Rgba<f32>,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let size = texture.size().as_f32() * pixel_scale(framebuffer);
        let pos = geng_utils::layout::align_aabb(size, quad, vec2(0.5, 0.5));
        self.geng.draw2d().textured_quad(
            framebuffer,
            &geng::PixelPerfectCamera,
            pos,
            texture,
            color,
        );
    }

    pub fn draw_ui(&self, model: &Model, framebuffer: &mut ugli::Framebuffer) {
        let frame_view = framebuffer.size().as_f32();
        let game_view = Aabb2::point(model.camera.center).extend_symmetric(
            vec2(framebuffer.size().as_f32().aspect(), 1.0) * model.camera.fov / 2.0,
        );

        if model.player.health.is_min() {
            // Death screen

            let pos = game_view.center() + vec2(0.0, 0.05) * game_view.size();
            let text = format!(
                "You Died\nScore: {}\nRooms cleared: {}\nBosses defeated: {}/2\n\nTry Again\nCtrl + R",
                model.score,
                model.rooms_cleared,
                model.bosses_killed
            );
            self.assets.font.draw(
                framebuffer,
                &model.camera,
                &text,
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(pos) * mat3::scale_uniform(1.5),
                self.assets.palette.text,
            );

            return;
        }

        // Score
        let pos = game_view.center() + vec2(0.0, 0.9) * game_view.size() / 2.0;
        self.assets.font.draw(
            framebuffer,
            &model.camera,
            &format!("SCORE: {}", model.score),
            vec2::splat(geng::TextAlign::CENTER),
            mat3::translate(pos) * mat3::scale_uniform(1.5),
            self.assets.palette.text,
        );
        if model.score_multiplier != R32::ONE {
            self.assets.font.draw(
                framebuffer,
                &model.camera,
                &format!("x{:.1}", model.score_multiplier),
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(pos - vec2(0.0, 1.5)) * mat3::scale_uniform(1.2),
                self.assets.palette.text,
            );
        }

        // Difficulty icon
        let pos = vec2(0.95, 0.95) * frame_view;
        let steps = [15.0, 35.0];
        let diff = model.difficulty.as_f32();
        let raw = model.difficulty_raw.as_f32();
        let (texture, next) = if diff < steps[0] {
            (
                &self.assets.sprites.easy,
                Some(Bounded::new(
                    raw,
                    model.config.difficulty.initial.as_f32()..=steps[0],
                )),
            )
        } else if diff < steps[1] {
            (
                &self.assets.sprites.medium,
                Some(Bounded::new(raw, steps[0]..=steps[1])),
            )
        } else {
            (&self.assets.sprites.hard, None)
        };
        self.draw_texture(
            Aabb2::point(pos),
            texture,
            self.assets.palette.text,
            framebuffer,
        );

        // Difficulty bar
        if let Some(fill_t) = next {
            let scale = pixel_scale(framebuffer);
            let bar = Aabb2::point(pos - vec2(0.0, scale * 15.0))
                .extend_symmetric(vec2(13.0, 3.0) * scale);
            self.geng.draw2d().quad(
                framebuffer,
                &geng::PixelPerfectCamera,
                bar,
                self.assets.palette.room,
            );

            let fill = bar.extend_uniform(-scale * 0.5);
            let fill_t = fill_t.get_ratio();
            let fill = fill.extend_right((fill_t - 1.0) * fill.width());
            self.geng.draw2d().quad(
                framebuffer,
                &geng::PixelPerfectCamera,
                fill,
                self.assets.palette.text,
            );
        }
    }

    pub fn draw_collider(
        &self,
        collider: &Collider,
        color: Rgba<f32>,
        camera: &Camera,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        self.draw_collider_transformed(mat3::identity(), collider, color, camera, framebuffer)
    }

    pub fn draw_collider_transformed(
        &self,
        transform: mat3<f32>,
        collider: &Collider,
        color: Rgba<f32>,
        camera: &Camera,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        let transform = collider.transform_mat().as_f32() * transform;
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
            Shape::Triangle { height } => {
                let height = height.as_f32();
                let base = height * 2.0 / 3.0.sqrt();
                let a = vec2(-base / 2.0, -height / 3.0);
                let b = vec2(base / 2.0, -height / 3.0);
                let c = vec2(0.0, height * 2.0 / 3.0);
                let vertices: Vec<_> = [a, b, c]
                    .into_iter()
                    .map(|pos| {
                        pos.rotate(collider.rotation.map(Coord::as_f32))
                            + collider.position.as_f32()
                    })
                    .collect();
                self.geng.draw2d().draw(
                    framebuffer,
                    camera,
                    &vertices,
                    color,
                    ugli::DrawMode::Triangles,
                );
            }
        }
    }

    pub fn draw_health_bar(
        &self,
        collider: &Collider,
        health: &Health,
        camera: &Camera,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        if health.is_max() || health.is_min() {
            return;
        }

        let aabb = collider.compute_aabb().map(Coord::as_f32);
        let health_bar = Aabb2::point(vec2(aabb.center().x, aabb.max.y + 0.2))
            .extend_symmetric(vec2(0.9, 0.2) / 2.0);

        // Outline
        self.geng
            .draw2d()
            .quad(framebuffer, camera, health_bar, self.assets.palette.health);
        let health_bar = health_bar.extend_uniform(-0.02);
        // Background
        self.geng.draw2d().quad(
            framebuffer,
            camera,
            health_bar,
            self.assets.palette.background,
        );
        // Fill
        let fill = health_bar.extend_symmetric(
            vec2(
                health_bar.width() * (health.get_ratio().as_f32() - 1.0),
                0.0,
            ) / 2.0,
        );
        self.geng
            .draw2d()
            .quad(framebuffer, camera, fill, self.assets.palette.health);
    }

    pub fn draw_outline(
        &self,
        collider: &Collider,
        outline_width: f32,
        color: Rgba<f32>,
        camera: &impl geng::AbstractCamera2d,
        framebuffer: &mut ugli::Framebuffer,
    ) {
        match collider.shape {
            Shape::Circle { radius } => {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    camera,
                    &draw2d::Ellipse::circle_with_cut(
                        collider.position.as_f32(),
                        radius.as_f32() - outline_width,
                        radius.as_f32(),
                        color,
                    ),
                );
            }
            Shape::Rectangle { width, height } => {
                let [a, b, c, d] = Aabb2::ZERO
                    .extend_symmetric(vec2(width.as_f32(), height.as_f32()) / 2.0)
                    .extend_uniform(-outline_width / 2.0)
                    .corners();
                let m = (a + b) / 2.0;
                self.geng.draw2d().draw2d(
                    framebuffer,
                    camera,
                    &draw2d::Chain::new(
                        Chain::new(vec![m, b, c, d, a, m]),
                        outline_width,
                        color,
                        1,
                    )
                    .rotate(collider.rotation.map(Coord::as_f32))
                    .translate(collider.position.as_f32()),
                );
            }
            Shape::Triangle { height } => {
                let height = height.as_f32();
                let base = height * 2.0 / 3.0.sqrt();
                let a = vec2(-base / 2.0, -height / 3.0);
                let b = vec2(base / 2.0, -height / 3.0);
                let c = vec2(0.0, height * 2.0 / 3.0);
                let m = (a + b) / 2.0;
                self.geng.draw2d().draw2d(
                    framebuffer,
                    camera,
                    &draw2d::Chain::new(Chain::new(vec![m, b, c, a, m]), outline_width, color, 1)
                        .rotate(collider.rotation.map(Coord::as_f32))
                        .translate(collider.position.as_f32()),
                );
            }
        }
    }
}

fn pixel_scale(framebuffer: &ugli::Framebuffer) -> f32 {
    framebuffer.size().y as f32 / crate::GAME_RESOLUTION.y as f32
}

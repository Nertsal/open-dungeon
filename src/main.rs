mod assets;
mod game;
mod model;
mod prelude;
mod render;

use self::assets::Assets;

use anyhow::Result;
use geng::prelude::*;

const FIXED_FPS: f64 = 60.0;

#[derive(clap::Parser)]
struct Opts {
    #[clap(flatten)]
    geng: geng::CliArgs,
}

fn main() {
    let opts: Opts = clap::Parser::parse();

    logger::init();
    geng::setup_panic_handler();

    let mut options = geng::ContextOptions::default();
    options.with_cli(&opts.geng);
    options.window.title = "Open Island".into();
    options.fixed_delta_time = 1.0 / FIXED_FPS;

    Geng::run_with(&options, |geng| async move {
        if let Err(err) = geng_main(geng).await {
            log::error!("application failed: {:?}", err);
        }
    });
}

async fn geng_main(geng: Geng) -> Result<()> {
    let manager = geng.asset_manager();

    let assets_path = run_dir().join("assets");
    let assets: Rc<Assets> = geng::asset::Load::load(manager, &assets_path, &()).await?;

    let state = game::GameState::new(&geng, &assets);
    geng.run_state(state).await;

    Ok(())
}

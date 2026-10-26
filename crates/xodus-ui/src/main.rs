mod app;
mod components;
mod models;
mod sections;
mod services;
mod state;
mod theme;

use app::GameLauncher;
use eframe::egui;

 fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1000.0])
            .with_min_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "xodus-ui // technical test 2 // BE 2026",
        options,
        Box::new(|cc| Ok(Box::new(GameLauncher::new(cc)))),
    )
}
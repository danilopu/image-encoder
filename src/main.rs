// main.rs
mod app;
mod utils;

use app::App;
use eframe::NativeOptions;

fn main() {
    let native_options = NativeOptions {
        initial_window_size: Some(egui::Vec2::new(1000.0, 700.0)),
        resizable: true,
        ..Default::default()
    };
    eframe::run_native(
        "JPEG to WebP Converter",
        native_options,
        Box::new(|_cc| Box::new(App::default())),
    );
}

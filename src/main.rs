mod app;
mod core;
mod decode;
mod detect;
mod samples;
mod settings;
mod theme;
mod ui;

use app::Basie64App;
use eframe::egui;

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([400.0, 520.0])
        .with_title("Basie-64")
        .with_decorations(false)
        .with_transparent(true);
    if let Some(icon) = theme::load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Basie-64",
        native_options,
        Box::new(|cc| {
            let app = Basie64App::default();
            theme::apply(&cc.egui_ctx, app.settings.theme);
            Ok(Box::new(app))
        }),
    )
}

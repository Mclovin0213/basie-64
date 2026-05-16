mod app;
mod decode;
mod detect;
#[cfg(target_os = "macos")]
mod macos_vibrancy;
mod samples;
mod settings;
mod theme;
mod ui;

// Re-export lib core module so binary modules can use `crate::core::*`.
pub use basie_64::core;

use app::Basie64App;
use eframe::egui;

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([400.0, 520.0])
        .with_min_inner_size([320.0, 400.0])
        .with_title("Basie-64")
        .with_transparent(true)
        .with_resizable(true);

    // On macOS we keep decorations enabled so the native traffic-light buttons
    // remain available — `with_decorations(false)` makes a borderless window
    // and strips them entirely. `fullsize_content_view` + `titlebar_shown(false)`
    // hides the visual titlebar while keeping the buttons. The OS also clips
    // the window to its native rounded shape automatically.
    //
    // On Windows / Linux we go fully borderless and rely on our painted
    // rounded-rect background + custom resize handles for the same look.
    #[cfg(target_os = "macos")]
    {
        viewport = viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_titlebar_buttons_shown(true)
            .with_title_shown(false);
    }
    #[cfg(not(target_os = "macos"))]
    {
        viewport = viewport.with_decorations(false);
    }

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
            theme::install_fonts(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx, app.settings.theme, app.settings.private_mode);

            // Native macOS vibrancy (`window_vibrancy::apply_vibrancy`) is
            // deferred until the first `update()` frame — see
            // `src/macos_vibrancy.rs`. Calling it here blanks the UI because
            // the NSVisualEffectView lands above the wgpu Metal layer before
            // it's realized. Controlled by the experimental
            // `experimental_native_vibrancy` setting (default off).
            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::apply_mica;
                let _ = apply_mica(cc, None);
            }

            Ok(Box::new(app))
        }),
    )
}

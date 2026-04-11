use crate::app::Basie64App;
use crate::theme;
use eframe::egui;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel")
        .frame(
            egui::Frame::default()
                .fill(theme::top_bar_fill(app.settings.theme))
                .inner_margin(8.0),
        )
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let resp = ui.interact(rect, ui.id().with("drag"), egui::Sense::drag());
            if resp.dragged() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            ui.horizontal(|ui| {
                ui.heading("🎷 Basie-64");
                ui.separator();
                ui.label("Encode / Decode");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("❌").on_hover_text("Close window").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    let theme_btn = ui
                        .button(app.settings.theme.label())
                        .on_hover_text("Cycle theme: Light → Dark → System");
                    if theme_btn.clicked() {
                        app.settings.theme = app.settings.theme.next();
                        app.settings.save();
                    }
                });
            });
        });
}

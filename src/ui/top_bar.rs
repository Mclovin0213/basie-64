use crate::app::Basie64App;
use crate::theme::{icons, Theme, Tokens};
use crate::ui::widgets;
use eframe::egui;

const BAR_HEIGHT: f32 = 48.0;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let tokens = Tokens::for_theme(app.settings.theme);

    let frame = egui::Frame::new()
        .fill(tokens.bg_surface)
        .inner_margin(egui::Margin {
            left: 16,
            right: 16,
            top: 0,
            bottom: 0,
        });

    egui::TopBottomPanel::top("top_panel")
        .frame(frame)
        .exact_height(BAR_HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .show(ctx, |ui| {
            let bar_rect = ui.max_rect();
            let drag = ui.interact(bar_rect, ui.id().with("drag"), egui::Sense::drag());
            if drag.dragged() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                logo_tile(ui, &tokens);
                ui.label(
                    egui::RichText::new("Basie-64")
                        .font(egui::FontId::new(
                            15.0,
                            egui::FontFamily::Name("inter_semibold".into()),
                        ))
                        .color(tokens.text_primary),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    if widgets::icon_button(ui, icons::X, "Close window", false).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    let history_btn = widgets::icon_button(
                        ui,
                        icons::HISTORY,
                        "Toggle history panel (⌘H)",
                        app.show_history_panel,
                    );
                    if history_btn.clicked() {
                        app.show_history_panel = !app.show_history_panel;
                    }

                    let settings_btn = widgets::icon_button(ui, icons::SETTINGS, "Settings", false);
                    let popup_id = ui.make_persistent_id("settings_menu");
                    if settings_btn.clicked() {
                        ui.memory_mut(|m| m.toggle_popup(popup_id));
                    }
                    egui::popup::popup_below_widget(
                        ui,
                        popup_id,
                        &settings_btn,
                        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                        |ui| {
                            ui.set_min_width(200.0);
                            let toggle =
                                ui.checkbox(&mut app.settings.private_mode, "Private mode");
                            if toggle.clicked() {
                                app.set_private_mode(app.settings.private_mode);
                            }
                        },
                    );

                    let theme_glyph = theme_icon(app.settings.theme);
                    let theme_tooltip = "Cycle theme: Light → Dark → System";
                    if widgets::icon_button(ui, theme_glyph, theme_tooltip, false).clicked() {
                        app.settings.theme = app.settings.theme.next();
                        app.settings.save();
                    }
                });
            });
        });
}

/// Square accent-blue logo tile with a centered Lucide BINARY glyph.
fn logo_tile(ui: &mut egui::Ui, t: &Tokens) {
    let size = egui::Vec2::splat(24.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(6), t.accent_blue);
    let galley = painter.layout_no_wrap(
        icons::BINARY.to_string(),
        egui::FontId::new(14.0, egui::FontFamily::Name("lucide".into())),
        t.btn_primary_text,
    );
    let pos = rect.center() - galley.size() * 0.5;
    painter.galley(pos, galley, t.btn_primary_text);
}

fn theme_icon(theme: Theme) -> char {
    match theme {
        Theme::Light => icons::SUN,
        Theme::Dark => icons::MOON,
        Theme::System => icons::MONITOR,
    }
}

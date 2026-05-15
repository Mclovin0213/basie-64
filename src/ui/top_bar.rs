use crate::app::Basie64App;
use crate::theme::{icons, Theme, Tokens};
use crate::ui::widgets;
use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, RichText, Sense, Stroke, Vec2,
};

const BAR_HEIGHT: f32 = 48.0;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let tokens = if app.settings.private_mode {
        Tokens::for_theme(app.settings.theme).with_private_tint()
    } else {
        Tokens::for_theme(app.settings.theme)
    };

    // On macOS the native traffic-light buttons sit in the top-left ~78pt of
    // the window when `fullsize_content_view` is on — push our content right
    // so the logo doesn't sit underneath them.
    let left_inset: i8 = if cfg!(target_os = "macos") { 88 } else { 16 };

    // Paint the semi-transparent window background here, rounded on the top
    // two corners only, so this panel forms the rounded window shape at the
    // head of the window.
    let radius = crate::app::WINDOW_RADIUS as u8;
    let frame = egui::Frame::new()
        .fill(tokens.window_fill(app.settings.translucent_window))
        .corner_radius(egui::CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        })
        .inner_margin(egui::Margin {
            left: left_inset,
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

                    // Native macOS traffic-light buttons (top-left) handle window
                    // close/minimize/maximize there. Only render our custom close
                    // button on Windows / Linux where there's no native chrome.
                    #[cfg(not(target_os = "macos"))]
                    {
                        if widgets::icon_button(ui, icons::X, "Close window", false).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
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
                            let mut translucent = app.settings.translucent_window;
                            if ui
                                .checkbox(&mut translucent, "Translucent window")
                                .on_hover_text("When on, the desktop bleeds through panel fills.")
                                .changed()
                            {
                                app.settings.translucent_window = translucent;
                                app.settings.save();
                                ctx.request_repaint();
                            }
                        },
                    );

                    // Private mode toggle
                    let pm_icon = if app.settings.private_mode {
                        icons::EYE_OFF
                    } else {
                        icons::EYE
                    };
                    let pm_tip = if app.settings.private_mode {
                        "Private mode ON \u{2014} history paused (\u{2318}\u{21E7}P)"
                    } else {
                        "Private mode OFF (\u{2318}\u{21E7}P)"
                    };
                    if private_mode_button(ui, pm_icon, pm_tip, app.settings.private_mode, &tokens)
                        .clicked()
                    {
                        app.set_private_mode(!app.settings.private_mode);
                    }
                    if app.settings.private_mode {
                        ui.label(
                            RichText::new("Private")
                                .font(FontId::new(11.0, FontFamily::Name("inter_semibold".into())))
                                .color(tokens.accent_purple),
                        );
                    }

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

fn private_mode_button(
    ui: &mut egui::Ui,
    glyph: char,
    tooltip: &str,
    active: bool,
    t: &Tokens,
) -> egui::Response {
    let size = Vec2::splat(32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let fill = if active {
            t.accent_purple_dim
        } else if response.hovered() || response.is_pointer_button_down_on() {
            t.bg_hover
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, CornerRadius::same(6), fill);
        if active {
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0, t.accent_purple),
                egui::StrokeKind::Inside,
            );
        }
        let glyph_color = if active {
            t.accent_purple
        } else {
            t.text_secondary
        };
        let galley = painter.layout_no_wrap(
            glyph.to_string(),
            FontId::new(16.0, FontFamily::Name("lucide".into())),
            glyph_color,
        );
        let pos = rect.center() - galley.size() * 0.5;
        painter.galley(pos, galley, glyph_color);
    }
    response.on_hover_text(tooltip)
}

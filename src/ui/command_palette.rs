use crate::app::Basie64App;
use crate::core::command_registry;
use crate::core::hash;
use crate::core::history::{HistoryEntry, HistoryOp};
use crate::theme::{Theme, Tokens};
use eframe::egui;

const PALETTE_WIDTH: f32 = 400.0;
const PALETTE_MAX_HEIGHT: f32 = 300.0;
const ITEM_HEIGHT: f32 = 28.0;

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let mut close_palette = false;
    let mut execute_command: Option<&'static str> = None; // command id

    // Get filtered commands (clone to avoid borrow issues with the closure)
    let filtered: Vec<_> = command_registry::filter_commands(&app.command_palette_query)
        .into_iter()
        .map(|(_idx, cmd, score)| (cmd.id, score))
        .collect();

    // Clamp selection
    if !filtered.is_empty() && app.command_palette_selected >= filtered.len() {
        app.command_palette_selected = filtered.len() - 1;
    }

    let t = Tokens::for_theme(app.settings.theme);

    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -80.0))
        .fixed_size(egui::vec2(PALETTE_WIDTH, PALETTE_MAX_HEIGHT))
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .frame(
            egui::Frame::new()
                .fill(t.overlay_surface)
                .inner_margin(egui::Margin::same(8))
                .corner_radius(egui::CornerRadius::same(12))
                .shadow(t.shadow_lg),
        )
        .show(ctx, |ui| {
            // Search input
            let search_resp = ui.add(
                egui::TextEdit::singleline(&mut app.command_palette_query)
                    .hint_text("Type a command…")
                    .desired_width(PALETTE_WIDTH - 16.0)
                    .font(egui::TextStyle::Body),
            );

            // Auto-focus search when palette opens
            if app.command_palette_just_opened {
                search_resp.request_focus();
                app.command_palette_just_opened = false;
            }

            // Handle keyboard navigation — read all keys in one closure to
            // avoid multiple borrows and redundant InputState copies.
            if search_resp.has_focus() {
                let (arrow_down, arrow_up, enter, escape) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowDown),
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::Enter),
                        i.key_pressed(egui::Key::Escape),
                    )
                });

                if arrow_down {
                    if filtered.is_empty() {
                        app.command_palette_selected = 0;
                    } else {
                        app.command_palette_selected =
                            (app.command_palette_selected + 1) % filtered.len();
                    }
                }
                if arrow_up {
                    if filtered.is_empty() {
                        app.command_palette_selected = 0;
                    } else {
                        app.command_palette_selected =
                            (app.command_palette_selected + filtered.len() - 1) % filtered.len();
                    }
                }
                if enter && !filtered.is_empty() {
                    execute_command = Some(filtered[app.command_palette_selected].0);
                }
                if escape {
                    close_palette = true;
                }
            }

            ui.add_space(6.0);

            // Scrollable command list
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("No commands found.").weak());
                    } else {
                        let max_visible = (PALETTE_MAX_HEIGHT / ITEM_HEIGHT) as usize;
                        let scroll_height = (filtered.len() as f32 * ITEM_HEIGHT)
                            .min(max_visible as f32 * ITEM_HEIGHT);

                        egui::ScrollArea::vertical()
                            .max_height(scroll_height)
                            .show(ui, |ui| {
                                for (list_idx, (cmd_id, _score)) in filtered.iter().enumerate() {
                                    let is_selected = list_idx == app.command_palette_selected;
                                    let Some(cmd) =
                                        command_registry::COMMANDS.iter().find(|c| c.id == *cmd_id)
                                    else {
                                        continue;
                                    };
                                    let clicked = render_command_row(ui, cmd, is_selected);
                                    if clicked {
                                        execute_command = Some(cmd_id);
                                    }
                                }
                            });
                    }
                });

            // Footer hint
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("↵ execute  ↑↓ navigate  esc close")
                        .small()
                        .weak(),
                );
            });

            // Close on outside click detection
            if ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
                let pointer_pos = ui.input(|i| i.pointer.interact_pos());
                if let Some(pos) = pointer_pos {
                    let window_rect = ui.max_rect();
                    if !window_rect.contains(pos) {
                        close_palette = true;
                    }
                }
            }
        });

    // Execute command outside the UI closure
    if let Some(cmd_id) = execute_command {
        execute_command_by_id(app, cmd_id, ctx);
        close_palette = true;
    }

    if close_palette {
        app.show_command_palette = false;
        app.command_palette_query.clear();
        app.command_palette_selected = 0;
    }
}

fn render_command_row(
    ui: &mut egui::Ui,
    cmd: &command_registry::Command,
    is_selected: bool,
) -> bool {
    let bg_color = if is_selected {
        ui.visuals().selection.bg_fill.gamma_multiply(0.3)
    } else {
        egui::Color32::TRANSPARENT
    };

    // Reserve a Noop shape slot BEFORE laying out widgets so the background
    // fill is painted behind the text rather than on top of it.
    let bg_shape_idx = ui.painter().add(egui::Shape::Noop);

    let resp = ui
        .horizontal(|ui| {
            ui.label(cmd.name);

            if !cmd.shortcut_display.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(cmd.shortcut_display).small().weak());
                });
            }
        })
        .response;

    // Now that we have the actual consumed rect, fill in the background shape.
    if bg_color != egui::Color32::TRANSPARENT {
        ui.painter().set(
            bg_shape_idx,
            egui::Shape::rect_filled(resp.rect, 2.0, bg_color),
        );
    }

    if is_selected {
        resp.scroll_to_me(Some(egui::Align::Center));
    }

    resp.clicked()
}

// ---------------------------------------------------------------------------
// Command execution — dispatches by stable command id string.
// The id strings must match the `id` fields in command_registry::COMMANDS.
// ---------------------------------------------------------------------------

fn execute_command_by_id(app: &mut Basie64App, cmd_id: &str, ctx: &egui::Context) {
    match cmd_id {
        "encode" => {
            app.run_encode();
            app.history_store.append(HistoryEntry::new(
                HistoryOp::Encode,
                &app.input,
                &app.output,
                "standard",
            ));
        }
        "decode" => {
            app.request_decode(ctx);
        }
        "copy_output" => {
            if !app.output.is_empty() {
                ctx.copy_text(app.output.clone());
                app.mark_copy_pulse();
            }
        }
        "clear_all" => {
            app.clear();
        }
        "toggle_theme" => {
            app.settings.theme = app.settings.theme.next();
            app.settings.save();
        }
        "theme_light" => {
            app.settings.theme = Theme::Light;
            app.settings.save();
        }
        "theme_dark" => {
            app.settings.theme = Theme::Dark;
            app.settings.save();
        }
        "theme_system" => {
            app.settings.theme = Theme::System;
            app.settings.save();
        }
        "open_history" => {
            app.show_history_panel = true;
        }
        "toggle_private" => {
            let new_mode = !app.settings.private_mode;
            app.set_private_mode(new_mode);
        }
        "batch_encode_folder" => {
            if !app.is_batch_running() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.start_batch_encode(dir, None);
                }
            }
        }
        "batch_decode_folder" => {
            if !app.is_batch_running() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.start_batch_decode(dir, None);
                }
            }
        }
        "show_diff_mode" => {
            app.open_diff_view_from_input();
        }
        "copy_md5" => {
            let hash_hex = hash::md5_hex(app.input.as_bytes());
            ctx.copy_text(hash_hex);
            app.mark_copy_pulse();
        }
        "copy_sha256" => {
            let hash_hex = hash::sha256_hex(app.input.as_bytes());
            ctx.copy_text(hash_hex);
            app.mark_copy_pulse();
        }
        "export_image" => {
            app.open_export_image_dialog(ctx);
        }
        _ => {}
    }
}

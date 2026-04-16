use crate::app::Basie64App;
use crate::theme::icons;
use crate::ui::widgets;
use eframe::egui;

const COPY_PULSE_DURATION: f64 = 0.35;

pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    widgets::input_frame(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                widgets::section_header(ui, "Output");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let copy_label = copy_button_label(app);
                    if widgets::secondary_button(ui, copy_label, Some(icons::COPY)).clicked() {
                        ctx.copy_text(app.output.clone());
                        app.mark_copy_pulse();
                    }
                    if let Some(data_uri) = &app.encoded_data_uri {
                        if widgets::ghost_button(ui, "Copy as Data URI", None).clicked() {
                            ctx.copy_text(data_uri.clone());
                            app.mark_copy_pulse();
                        }
                    }
                });
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt("output_scroll")
                .max_height(140.0)
                .show(ui, |ui| {
                    let t = widgets::tokens(ui);
                    ui.add(
                        egui::TextEdit::multiline(&mut app.output)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false)
                            .desired_width(f32::INFINITY)
                            .desired_rows(6)
                            .text_color(t.text_mono),
                    );
                });

            if let Some(texture) = app.image_preview.clone() {
                ui.add_space(8.0);
                image_meta_bar::show(ui, app, texture);
            }
        });
    });

    // JWT inspector card — outside the main Output frame so it gets its own card.
    if let Some(insp) = app.jwt_inspection.clone() {
        ui.add_space(8.0);
        jwt_inspector::show(ui, app, &insp);
    }

    // Clear expired pulse
    if let Some(started) = app.copy_pulse_at {
        if app.now - started > COPY_PULSE_DURATION {
            app.copy_pulse_at = None;
        }
    }
}

fn copy_button_label(app: &Basie64App) -> &'static str {
    match app.copy_pulse_at {
        Some(started) if app.now - started < COPY_PULSE_DURATION => "Copied!",
        _ => "Copy",
    }
}

mod image_meta_bar {
    use crate::app::Basie64App;
    use crate::theme::icons;
    use crate::ui::widgets;
    use eframe::egui::{self, CornerRadius, Frame, Margin};

    pub fn show(ui: &mut egui::Ui, app: &mut Basie64App, texture: egui::TextureHandle) {
        let Some(meta) = app.image_meta.clone() else {
            return;
        };
        let t = widgets::tokens(ui);

        // (1) Image preview with top-only rounded corners
        Frame::new()
            .fill(t.bg_card)
            .corner_radius(CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            })
            .shadow(t.shadow_sm)
            .show(ui, |ui| {
                ui.add(egui::Image::new(&texture).max_width(ui.available_width()));
            });

        // (2) Metadata bar: kind chip + dimensions + size + Export button
        Frame::new()
            .fill(t.bg_card)
            .corner_radius(CornerRadius::same(0))
            .inner_margin(Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    widgets::meta_chip(ui, meta.kind.label());
                    ui.add_space(8.0);
                    let t2 = widgets::tokens(ui);
                    ui.label(
                        egui::RichText::new(format!("{}×{}", meta.width, meta.height))
                            .font(egui::FontId::monospace(12.0))
                            .color(t2.text_secondary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(humanize_size(meta.size_bytes))
                            .font(egui::FontId::monospace(12.0))
                            .color(t2.text_muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::secondary_button(ui, "Export…", Some(icons::DOWNLOAD))
                            .on_hover_text("Open the Export Image dialog")
                            .clicked()
                        {
                            let ctx = ui.ctx().clone();
                            app.open_export_image_dialog(&ctx);
                        }
                    });
                });
            });

        // (3) EXIF card
        if !meta.exif.is_empty() {
            ui.add_space(4.0);
            widgets::card_frame(ui, |ui| {
                let t2 = widgets::tokens(ui);
                let collapse_id = ui.id().with("image_meta_exif_collapse");
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    collapse_id,
                    false,
                )
                .show_header(ui, |ui| {
                    let t3 = widgets::tokens(ui);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} EXIF {}",
                            meta.exif.len(),
                            if meta.exif.len() == 1 {
                                "field"
                            } else {
                                "fields"
                            }
                        ))
                        .color(t3.accent_amber),
                    );
                })
                .body(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("image_meta_exif_scroll")
                        .max_height(160.0)
                        .show(ui, |ui| {
                            let t3 = widgets::tokens(ui);
                            egui::Grid::new("image_meta_exif_grid")
                                .num_columns(2)
                                .spacing([12.0, 2.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for field in &meta.exif {
                                        ui.label(
                                            egui::RichText::new(&field.tag)
                                                .font(egui::FontId::monospace(11.0))
                                                .color(t3.text_secondary),
                                        );
                                        ui.label(
                                            egui::RichText::new(&field.value)
                                                .font(egui::FontId::monospace(11.0))
                                                .color(t3.text_primary),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });

                // Bottom status line
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Decoded · {} · {}",
                        meta.kind.label(),
                        humanize_size(meta.size_bytes)
                    ))
                    .font(egui::FontId::monospace(11.0))
                    .color(t2.text_muted),
                );
            });
        } else if meta.has_strippable_metadata {
            ui.add_space(4.0);
            widgets::card_frame(ui, |ui| {
                let t2 = widgets::tokens(ui);
                ui.label(
                    egui::RichText::new("Non-EXIF metadata present (text chunks / XMP / IPTC).")
                        .color(t2.accent_amber)
                        .small(),
                );
            });
        }
    }

    fn humanize_size(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;
        const GB: usize = MB * 1024;
        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

mod jwt_inspector {
    use crate::app::Basie64App;
    use crate::core::jwt::{
        explain_claim, format_epoch_utc, format_relative, verify_hmac, JwtInspection, JwtWarning,
        VerificationResult,
    };
    use crate::theme::icons;
    use crate::ui::widgets::{self, AccentTone};
    use eframe::egui;

    pub fn show(ui: &mut egui::Ui, app: &mut Basie64App, insp: &JwtInspection) {
        widgets::card_frame(ui, |ui| {
            ui.vertical(|ui| {
                // ── Header section ────────────────────────────────────────
                widgets::section_header(ui, "Header");
                ui.add_space(6.0);
                header_grid(ui, insp);

                ui.add_space(6.0);
                widgets::divider(ui);
                ui.add_space(6.0);

                // ── Payload section ───────────────────────────────────────
                widgets::section_header(ui, "Payload");
                ui.add_space(6.0);
                payload_grid(ui, insp);

                // ── Warnings strip ────────────────────────────────────────
                if !insp.warnings.is_empty() {
                    ui.add_space(8.0);
                    widgets::divider(ui);
                    ui.add_space(8.0);
                    for w in &insp.warnings {
                        let text = warning_text(w);
                        widgets::accent_banner(
                            ui,
                            AccentTone::Orange,
                            Some(icons::TRIANGLE_ALERT),
                            &text,
                            None,
                        );
                        ui.add_space(4.0);
                    }
                }

                // ── HMAC verify section ───────────────────────────────────
                ui.add_space(8.0);
                widgets::divider(ui);
                ui.add_space(8.0);
                verify_section(ui, app, insp);
            });
        });
    }

    fn header_grid(ui: &mut egui::Ui, insp: &JwtInspection) {
        let t = widgets::tokens(ui);
        egui::Grid::new("jwt_header_grid")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("alg")
                        .font(egui::FontId::monospace(12.0))
                        .color(t.text_secondary),
                );
                ui.label(
                    egui::RichText::new(if insp.header.alg.is_empty() {
                        "(missing)"
                    } else {
                        insp.header.alg.as_str()
                    })
                    .font(egui::FontId::monospace(12.0))
                    .color(t.text_primary),
                );
                ui.end_row();
                if let Some(typ) = &insp.header.typ {
                    ui.label(
                        egui::RichText::new("typ")
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(typ)
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_primary),
                    );
                    ui.end_row();
                }
                if let Some(kid) = &insp.header.kid {
                    ui.label(
                        egui::RichText::new("kid")
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(kid)
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_primary),
                    );
                    ui.end_row();
                }
                for (k, v) in &insp.header.extra {
                    ui.label(
                        egui::RichText::new(k)
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(v.to_string())
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_primary),
                    );
                    ui.end_row();
                }
            });
    }

    fn payload_grid(ui: &mut egui::Ui, insp: &JwtInspection) {
        let t = widgets::tokens(ui);
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        egui::Grid::new("jwt_payload_grid")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                for (k, v) in &insp.payload {
                    ui.label(
                        egui::RichText::new(k)
                            .font(egui::FontId::monospace(12.0))
                            .color(t.text_secondary),
                    );
                    ui.vertical(|ui| {
                        let t2 = widgets::tokens(ui);
                        let is_time_claim = matches!(k.as_str(), "exp" | "iat" | "nbf");
                        if is_time_claim {
                            if let Some(epoch) = v.as_i64() {
                                let delta = epoch - now_secs;
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} → {} ({})",
                                        epoch,
                                        format_epoch_utc(epoch),
                                        format_relative(delta)
                                    ))
                                    .font(egui::FontId::monospace(12.0))
                                    .color(t2.text_primary),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new(v.to_string())
                                        .font(egui::FontId::monospace(12.0))
                                        .color(t2.text_primary),
                                );
                            }
                        } else {
                            ui.label(
                                egui::RichText::new(value_to_display(v))
                                    .font(egui::FontId::monospace(12.0))
                                    .color(t2.text_primary),
                            );
                        }
                        if let Some(explanation) = explain_claim(k) {
                            ui.label(
                                egui::RichText::new(explanation)
                                    .small()
                                    .color(t2.text_muted),
                            );
                        }
                    });
                    ui.end_row();
                }
            });
    }

    fn value_to_display(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    fn warning_text(w: &JwtWarning) -> String {
        match w {
            JwtWarning::AlgNone => {
                "alg: none — this token is unsigned, do not trust it".to_string()
            }
            JwtWarning::Expired { exp, ago_secs } => {
                format!("Expired: exp={} ({})", exp, format_relative(-(*ago_secs)))
            }
            JwtWarning::NotYetValid { nbf, in_secs } => {
                format!("Not yet valid: nbf={} ({})", nbf, format_relative(*in_secs))
            }
            JwtWarning::IssuedInFuture { iat, in_secs } => {
                format!(
                    "Issued in the future: iat={} ({})",
                    iat,
                    format_relative(*in_secs)
                )
            }
            JwtWarning::MissingExp => "No `exp` claim — this token never expires".to_string(),
            JwtWarning::MalformedTimestamp { claim } => {
                format!("`{claim}` is not a valid integer timestamp")
            }
        }
    }

    fn verify_section(ui: &mut egui::Ui, app: &mut Basie64App, insp: &JwtInspection) {
        let collapse_id = ui.id().with("jwt_verify_section");
        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            collapse_id,
            false,
        )
        .show_header(ui, |ui| {
            widgets::section_header(ui, "Verify Signature (HMAC)");
        })
        .body(|ui| {
            let t = widgets::tokens(ui);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Paste the HMAC secret to verify the signature locally. \
                     RS256/ES256 (asymmetric) algorithms are not yet supported.",
                )
                .small()
                .color(t.text_muted),
            );
            ui.add_space(8.0);
            widgets::input_frame(ui, |ui| {
                ui.horizontal(|ui| {
                    let t2 = widgets::tokens(ui);
                    ui.label(
                        egui::RichText::new("Secret")
                            .color(t2.text_secondary)
                            .small(),
                    );
                    ui.add_space(4.0);
                    let available = (ui.available_width() - 120.0).max(120.0);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut app.jwt_secret_input)
                            .desired_width(available)
                            .font(egui::TextStyle::Monospace),
                    );
                    if edit.changed() {
                        app.jwt_verification = None;
                    }
                    if widgets::ghost_button(ui, "Verify Signature", Some(icons::SHIELD_CHECK))
                        .clicked()
                    {
                        app.jwt_verification =
                            Some(verify_hmac(insp, app.jwt_secret_input.as_bytes()));
                    }
                });
            });
            if let Some(result) = &app.jwt_verification {
                ui.add_space(6.0);
                let (tone, text) = result_display(result);
                widgets::accent_banner(ui, tone, None, &text, None);
            }
        });
    }

    fn result_display(r: &VerificationResult) -> (AccentTone, String) {
        match r {
            VerificationResult::Ok => (AccentTone::Green, "Signature valid".to_string()),
            VerificationResult::Mismatch => {
                (AccentTone::Red, "Signature does not match".to_string())
            }
            VerificationResult::UnsupportedAlg(alg) => (
                AccentTone::Amber,
                format!(
                    "Algorithm `{alg}` is not supported. \
                     HMAC verification covers HS256, HS384, and HS512."
                ),
            ),
            VerificationResult::InvalidSignatureEncoding => (
                AccentTone::Red,
                "Signature is not valid base64url".to_string(),
            ),
            VerificationResult::EmptySecret => (
                AccentTone::Amber,
                "Enter a secret to verify the signature".to_string(),
            ),
        }
    }
}

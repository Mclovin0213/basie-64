use crate::app::Basie64App;
use eframe::egui;

const COPY_PULSE_DURATION: f64 = 0.35;

pub fn show(app: &mut Basie64App, ctx: &egui::Context, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Output");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let copy_label = copy_button_label(app);
                    let copy_resp = ui
                        .button(copy_label)
                        .on_hover_text("Copy output to clipboard (⌘⇧C)");
                    if copy_resp.clicked() {
                        ctx.copy_text(app.output.clone());
                        app.mark_copy_pulse();
                    }
                    if let Some(data_uri) = &app.encoded_data_uri {
                        if ui
                            .button("🌐 Copy as Data URI")
                            .on_hover_text("Copy as data: URI for embedding in web content")
                            .clicked()
                        {
                            ctx.copy_text(data_uri.clone());
                            app.mark_copy_pulse();
                        }
                    }
                });
            });
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .id_salt("output_scroll")
                .max_height(140.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.output)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false)
                            .desired_width(f32::INFINITY)
                            .desired_rows(6),
                    );
                });

            if let Some(texture) = &app.image_preview {
                ui.add_space(8.0);
                ui.label("Image Preview:");
                ui.add(egui::Image::new(texture).max_width(ui.available_width()));
            }
        });
    });

    // JWT inspector card — lives outside the main Output frame so it gets
    // its own visual group.
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
        Some(started) if app.now - started < COPY_PULSE_DURATION => "✓ Copied!",
        _ => "📋 Copy",
    }
}

mod jwt_inspector {
    use crate::app::Basie64App;
    use crate::core::jwt::{
        explain_claim, format_epoch_utc, format_relative, verify_hmac, JwtInspection, JwtWarning,
        VerificationResult,
    };
    use eframe::egui;

    const WARN_COLOR: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x9F, 0x3E);
    const OK_COLOR: egui::Color32 = egui::Color32::from_rgb(0x4C, 0xAF, 0x50);
    const ERR_COLOR: egui::Color32 = egui::Color32::from_rgb(0xE5, 0x73, 0x73);

    pub fn show(ui: &mut egui::Ui, app: &mut Basie64App, insp: &JwtInspection) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.heading("🔐 JWT Inspector");
                ui.add_space(4.0);

                ui.label(egui::RichText::new("Header").strong());
                header_grid(ui, insp);

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Payload").strong());
                payload_grid(ui, insp);

                if !insp.warnings.is_empty() {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Warnings").strong());
                    warnings_box(ui, &insp.warnings);
                }

                ui.add_space(8.0);
                verify_section(ui, app, insp);
            });
        });
    }

    fn header_grid(ui: &mut egui::Ui, insp: &JwtInspection) {
        egui::Grid::new("jwt_header_grid")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("alg").monospace());
                ui.label(
                    egui::RichText::new(if insp.header.alg.is_empty() {
                        "(missing)"
                    } else {
                        insp.header.alg.as_str()
                    })
                    .monospace(),
                );
                ui.end_row();
                if let Some(typ) = &insp.header.typ {
                    ui.label(egui::RichText::new("typ").monospace());
                    ui.label(egui::RichText::new(typ).monospace());
                    ui.end_row();
                }
                if let Some(kid) = &insp.header.kid {
                    ui.label(egui::RichText::new("kid").monospace());
                    ui.label(egui::RichText::new(kid).monospace());
                    ui.end_row();
                }
                for (k, v) in &insp.header.extra {
                    ui.label(egui::RichText::new(k).monospace());
                    ui.label(egui::RichText::new(v.to_string()).monospace());
                    ui.end_row();
                }
            });
    }

    fn payload_grid(ui: &mut egui::Ui, insp: &JwtInspection) {
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
                    ui.label(egui::RichText::new(k).monospace());
                    ui.vertical(|ui| {
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
                                    .monospace(),
                                );
                            } else {
                                ui.label(egui::RichText::new(v.to_string()).monospace());
                            }
                        } else {
                            ui.label(egui::RichText::new(value_to_display(v)).monospace());
                        }
                        if let Some(explanation) = explain_claim(k) {
                            ui.label(egui::RichText::new(explanation).small().weak());
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

    fn warnings_box(ui: &mut egui::Ui, warnings: &[JwtWarning]) {
        for w in warnings {
            let (icon, text) = warning_display(w);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).color(WARN_COLOR));
                ui.label(egui::RichText::new(text).color(WARN_COLOR));
            });
        }
    }

    fn warning_display(w: &JwtWarning) -> (&'static str, String) {
        match w {
            JwtWarning::AlgNone => (
                "⚠",
                "alg: none — this token is unsigned, do not trust it".to_string(),
            ),
            JwtWarning::Expired { exp, ago_secs } => (
                "⚠",
                format!("Expired: exp={} ({})", exp, format_relative(-(*ago_secs))),
            ),
            JwtWarning::NotYetValid { nbf, in_secs } => (
                "⚠",
                format!("Not yet valid: nbf={} ({})", nbf, format_relative(*in_secs)),
            ),
            JwtWarning::IssuedInFuture { iat, in_secs } => (
                "⚠",
                format!(
                    "Issued in the future: iat={} ({})",
                    iat,
                    format_relative(*in_secs)
                ),
            ),
            JwtWarning::MissingExp => {
                ("ℹ", "No `exp` claim — this token never expires".to_string())
            }
            JwtWarning::MalformedTimestamp { claim } => {
                ("⚠", format!("`{claim}` is not a valid integer timestamp"))
            }
        }
    }

    fn verify_section(ui: &mut egui::Ui, app: &mut Basie64App, insp: &JwtInspection) {
        egui::CollapsingHeader::new("Verify signature (HMAC)")
            .id_salt("jwt_verify_section")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Paste the HMAC secret to verify the signature locally. \
                         RS256/ES256 (asymmetric) algorithms are not yet supported.",
                    )
                    .small()
                    .weak(),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Secret:");
                    let available = (ui.available_width() - 96.0).max(120.0);
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut app.jwt_secret_input)
                            .desired_width(available)
                            .font(egui::TextStyle::Monospace),
                    );
                    if edit.changed() {
                        app.jwt_verification = None;
                    }
                    if ui.button("Verify").clicked() {
                        app.jwt_verification =
                            Some(verify_hmac(insp, app.jwt_secret_input.as_bytes()));
                    }
                });
                if let Some(result) = &app.jwt_verification {
                    ui.add_space(4.0);
                    let (color, text) = result_display(result);
                    ui.label(egui::RichText::new(text).color(color).strong());
                }
            });
    }

    fn result_display(r: &VerificationResult) -> (egui::Color32, String) {
        match r {
            VerificationResult::Ok => (OK_COLOR, "✓ Signature valid".to_string()),
            VerificationResult::Mismatch => (ERR_COLOR, "✗ Signature does not match".to_string()),
            VerificationResult::UnsupportedAlg(alg) => (
                WARN_COLOR,
                format!(
                    "Algorithm `{alg}` is not supported. \
                     HMAC verification covers HS256, HS384, and HS512."
                ),
            ),
            VerificationResult::InvalidSignatureEncoding => {
                (ERR_COLOR, "Signature is not valid base64url".to_string())
            }
            VerificationResult::EmptySecret => (
                WARN_COLOR,
                "Enter a secret to verify the signature".to_string(),
            ),
        }
    }
}

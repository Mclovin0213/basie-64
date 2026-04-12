use crate::app::Basie64App;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    OpenDiffView,
    ToggleHistory,
    ClearAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandItem {
    pub title: &'static str,
    pub keywords: &'static str,
    pub shortcut: &'static str,
    pub action: CommandAction,
}

const COMMANDS: &[CommandItem] = &[
    CommandItem {
        title: "Open Diff View",
        keywords: "diff compare comparison",
        shortcut: "⌘D",
        action: CommandAction::OpenDiffView,
    },
    CommandItem {
        title: "Toggle History",
        keywords: "history recent entries",
        shortcut: "⌘H",
        action: CommandAction::ToggleHistory,
    },
    CommandItem {
        title: "Clear Input and Output",
        keywords: "clear reset escape",
        shortcut: "Esc",
        action: CommandAction::ClearAll,
    },
];

pub fn filtered_commands(query: &str) -> Vec<CommandItem> {
    let query = query.trim().to_ascii_lowercase();
    COMMANDS
        .iter()
        .copied()
        .filter(|item| {
            query.is_empty()
                || item.title.to_ascii_lowercase().contains(&query)
                || item.keywords.contains(&query)
        })
        .collect()
}

pub fn show(app: &mut Basie64App, ctx: &egui::Context) {
    let commands = filtered_commands(&app.command_palette_query);
    let mut chosen = None;

    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 48.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut app.command_palette_query)
                    .hint_text("Search commands…")
                    .desired_width(360.0),
            );
            if response.gained_focus() || !response.has_focus() {
                response.request_focus();
            }

            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                chosen = commands.first().map(|item| item.action);
            }

            ui.add_space(8.0);

            if commands.is_empty() {
                ui.label(egui::RichText::new("No commands match.").weak());
                return;
            }

            for item in commands {
                ui.horizontal(|ui| {
                    if ui.button(item.title).clicked() {
                        chosen = Some(item.action);
                    }
                    ui.label(egui::RichText::new(item.shortcut).weak().small());
                });
            }
        });

    if let Some(action) = chosen {
        app.apply_command_palette_action(action);
    }
}

#[cfg(test)]
mod tests {
    use super::{filtered_commands, CommandAction};

    #[test]
    fn filtered_commands_finds_diff_entry() {
        let commands = filtered_commands("diff");
        assert!(commands
            .iter()
            .any(|item| item.action == CommandAction::OpenDiffView));
    }
}

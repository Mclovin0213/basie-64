use eframe::egui::{
    self, Area, CursorIcon, Id, Order, Rect, ResizeDirection, Sense, ViewportCommand,
};

const EDGE: f32 = 6.0;
const CORNER: f32 = 12.0;

pub fn show(ctx: &egui::Context) {
    let screen = ctx.screen_rect();
    if screen.width() < CORNER * 2.0 || screen.height() < CORNER * 2.0 {
        return;
    }

    Area::new(Id::new("window_resize_handles"))
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());

            let zones = [
                (
                    Rect::from_min_max(
                        screen.left_top() + egui::vec2(CORNER, 0.0),
                        screen.right_top() + egui::vec2(-CORNER, EDGE),
                    ),
                    ResizeDirection::North,
                    CursorIcon::ResizeNorth,
                ),
                (
                    Rect::from_min_max(
                        screen.left_bottom() + egui::vec2(CORNER, -EDGE),
                        screen.right_bottom() + egui::vec2(-CORNER, 0.0),
                    ),
                    ResizeDirection::South,
                    CursorIcon::ResizeSouth,
                ),
                (
                    Rect::from_min_max(
                        screen.left_top() + egui::vec2(0.0, CORNER),
                        screen.left_bottom() + egui::vec2(EDGE, -CORNER),
                    ),
                    ResizeDirection::West,
                    CursorIcon::ResizeWest,
                ),
                (
                    Rect::from_min_max(
                        screen.right_top() + egui::vec2(-EDGE, CORNER),
                        screen.right_bottom() + egui::vec2(0.0, -CORNER),
                    ),
                    ResizeDirection::East,
                    CursorIcon::ResizeEast,
                ),
                (
                    Rect::from_min_size(screen.left_top(), egui::vec2(CORNER, CORNER)),
                    ResizeDirection::NorthWest,
                    CursorIcon::ResizeNwSe,
                ),
                (
                    Rect::from_min_size(
                        screen.right_top() + egui::vec2(-CORNER, 0.0),
                        egui::vec2(CORNER, CORNER),
                    ),
                    ResizeDirection::NorthEast,
                    CursorIcon::ResizeNeSw,
                ),
                (
                    Rect::from_min_size(
                        screen.left_bottom() + egui::vec2(0.0, -CORNER),
                        egui::vec2(CORNER, CORNER),
                    ),
                    ResizeDirection::SouthWest,
                    CursorIcon::ResizeNeSw,
                ),
                (
                    Rect::from_min_size(
                        screen.right_bottom() + egui::vec2(-CORNER, -CORNER),
                        egui::vec2(CORNER, CORNER),
                    ),
                    ResizeDirection::SouthEast,
                    CursorIcon::ResizeNwSe,
                ),
            ];

            for (i, (rect, dir, cursor)) in zones.into_iter().enumerate() {
                let id = Id::new(("window_resize_zone", i));
                let resp = ui.interact(rect, id, Sense::drag());
                if resp.hovered() || resp.dragged() {
                    ctx.set_cursor_icon(cursor);
                }
                if resp.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }
            }
        });
}

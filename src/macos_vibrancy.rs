//! macOS native vibrancy (NSVisualEffectView) installation, deferred until
//! after eframe's wgpu/Metal layer is fully realized.
//!
//! The bundled `window_vibrancy::apply_vibrancy(...)` call inserts the
//! `NSVisualEffectView` as a **subview of the Metal-backed `WinitView`**, so
//! the effect view's layer composites *above* the Metal layer — the UI
//! disappears. The fix is to add the effect view as a **sibling of** the
//! WinitView (under the same superview, positioned below), so the Metal
//! layer paints on top.
//!
//! See `~/.claude/plans/can-we-attempt-the-vivid-bird.md` for the full
//! write-up. Two strategies live here:
//!
//! - `try_install_deferred` calls `window_vibrancy::apply_vibrancy`. Simpler
//!   and what we tried first; observed to blank the UI on eframe 0.31 + wgpu.
//! - `try_install_native` does the manual `objc2` view-hierarchy surgery
//!   that actually produces a rendering UI with real OS-level blur.

#![cfg(target_os = "macos")]

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
};
use objc2_foundation::NSRect;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub fn try_install_native(frame: &eframe::Frame, radius: f64) -> Result<(), &'static str> {
    // SAFETY: `update()` is called on the main thread; eframe guarantees this
    // and we only invoke this function from `Basie64App::update`. The
    // `MainThreadMarker` is needed because most `NSView` APIs require it.
    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    let handle = frame
        .window_handle()
        .map_err(|_| "no window handle yet")?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return Err("not an AppKit window");
    };

    // The `ns_view` from `raw_window_handle` is winit's `WinitView` — the
    // Metal-backed view that hosts eframe's `CAMetalLayer`. We want to add a
    // sibling NSVisualEffectView under the same parent, positioned below it.
    //
    // SAFETY: `appkit.ns_view` is a valid `NSView` pointer for the lifetime
    // of the window, guaranteed by raw-window-handle's contract.
    let winit_view: Retained<NSView> = unsafe {
        let ptr: *mut NSView = appkit.ns_view.as_ptr().cast();
        Retained::retain(ptr).ok_or("ns_view was null")?
    };

    let parent: Retained<NSView> = unsafe { winit_view.superview() }
        .ok_or("winit view has no superview yet — too early to install vibrancy")?;

    let effect: Retained<NSVisualEffectView> = {
        let alloc = NSVisualEffectView::alloc(mtm);
        let bounds: NSRect = parent.bounds();
        NSVisualEffectView::initWithFrame(alloc, bounds)
    };

    effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
    effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    effect.setState(NSVisualEffectState::FollowsWindowActiveState);
    effect.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );

    if let Some(layer) = effect.layer() {
        layer.setCornerRadius(radius);
        layer.setMasksToBounds(true);
    }

    // The key difference from `window_vibrancy::apply_vibrancy`: we add the
    // effect view to `parent` (the WinitView's superview), positioned
    // *Below* `winit_view`, so the Metal layer composites on top of the
    // effect view's layer instead of underneath.
    parent.addSubview_positioned_relativeTo(
        &effect,
        NSWindowOrderingMode::Below,
        Some(&winit_view),
    );

    Ok(())
}

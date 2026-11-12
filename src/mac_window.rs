//! macOS: the blur behind a window of the app, found by its title (a `Window(..).separate()` is an OS window of
//! its own, so the blur of the main window does not reach it). Nothing happens on other systems.

#[cfg(target_os = "macos")]
mod imp {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindowOrderingMode};
    use std::cell::RefCell;
    use std::collections::HashMap;

    // The private call Terminal.app uses (egui_bg_layer's `set_window_blur` makes the same one for the main window).
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGSMainConnectionID() -> *mut std::ffi::c_void;
        fn CGSSetWindowBackgroundBlurRadius(connection: *mut std::ffi::c_void, window: isize, radius: i64) -> i32;
    }

    thread_local! {
        /// title -> (window number, radius) that was set last: a closed and reopened window has a new number.
        static DONE: RefCell<HashMap<String, (isize, i64)>> = RefCell::new(HashMap::new());
    }

    /// Makes the window with this title a child of the app's main window (AppKit keeps a child in front of its
    /// parent, hides and shows it with the parent, and moves it along). Already a child: nothing to do.
    pub fn attach(title: &str) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        let windows = app.windows();
        let mut child = None;
        for i in 0..windows.count() {
            let window = windows.objectAtIndex(i);
            if window.title().to_string() == title {
                child = Some(window);
                break;
            }
        }
        let Some(child) = child else {
            return;
        };
        if child.parentWindow().is_some() {
            return;
        }
        // the parent: the app's main window, else the first other window that is not itself a child
        let mut parent = app.mainWindow().filter(|w| w.title().to_string() != title && w.parentWindow().is_none());
        if parent.is_none() {
            for i in 0..windows.count() {
                let window = windows.objectAtIndex(i);
                if window.title().to_string() != title && window.parentWindow().is_none() && window.canBecomeMainWindow() && window.isVisible() {
                    parent = Some(window);
                    break;
                }
            }
        }
        if let Some(parent) = parent {
            // SAFETY: both are live windows of this app, and the call only links them.
            unsafe {
                parent.addChildWindow_ordered(&child, NSWindowOrderingMode::Above);
            }
        }
    }

    pub fn set_blur(title: &str, radius: i64) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let windows = NSApplication::sharedApplication(mtm).windows();
        for i in 0..windows.count() {
            let window = windows.objectAtIndex(i);
            if window.title().to_string() != title {
                continue;
            }
            let number = window.windowNumber();
            let same = DONE.with(|d| d.borrow().get(title).is_some_and(|&(n, r)| n == number && r == radius));
            if same {
                return;
            }
            // SAFETY: private but long-stable CoreGraphics call; `number` is the number of a live window of this app.
            unsafe {
                CGSSetWindowBackgroundBlurRadius(CGSMainConnectionID(), number, radius);
            }
            DONE.with(|d| d.borrow_mut().insert(title.to_string(), (number, radius)));
            return;
        }
    }
}

/// Blur `0..=100` behind the window with this title (does it once per window and radius; cheap to call every frame).
#[cfg(target_os = "macos")]
pub fn set_blur(title: &str, amount: u8) {
    imp::set_blur(title, i64::from(amount));
}

/// Other systems: nothing happens.
#[cfg(not(target_os = "macos"))]
pub fn set_blur(_title: &str, _amount: u8) {}

/// The window with this title becomes a child of the app window (always in front of it, minimised and restored
/// with it). Cheap to call every frame: it does nothing once the window is a child.
#[cfg(target_os = "macos")]
pub fn attach_to_app_window(title: &str) {
    imp::attach(title);
}

/// Other systems: nothing happens.
#[cfg(not(target_os = "macos"))]
pub fn attach_to_app_window(_title: &str) {}

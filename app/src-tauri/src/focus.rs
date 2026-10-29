//! Keeps the app from stealing focus from whatever the user is working in.
//!
//! The bar is a non-focusable window, but on macOS clicking it still activates
//! our app. So we remember the last app that was in front and hand focus back
//! to it after every interaction.

use std::{thread, time::Duration};

use tauri::{AppHandle, WebviewWindow};

/// Polls the frontmost app so we know where to send focus back to.
pub fn start_tracking(app: AppHandle) {
    thread::spawn(move || {
        loop {
            let _ = app.run_on_main_thread(remember_frontmost);
            thread::sleep(Duration::from_millis(400));
        }
    });
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::atomic::{AtomicI32, Ordering};

    use objc2::{MainThreadMarker, runtime::NSObjectProtocol, sel};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationOptions, NSRunningApplication, NSWindow,
        NSWindowCollectionBehavior, NSWorkspace,
    };

    use super::WebviewWindow;

    static LAST_PID: AtomicI32 = AtomicI32::new(0);

    pub fn remember_frontmost() {
        let Some(front) = NSWorkspace::sharedWorkspace().frontmostApplication() else {
            return;
        };
        let pid = front.processIdentifier();
        if pid != std::process::id() as i32 {
            LAST_PID.store(pid, Ordering::Relaxed);
        }
    }

    #[allow(deprecated)]
    pub fn restore_frontmost() {
        let pid = LAST_PID.load(Ordering::Relaxed);
        if pid == 0 {
            return;
        }
        let Some(target) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
            return;
        };
        // macOS 14+ uses cooperative activation: we have to yield before the other app can take over.
        if let Some(mtm) = MainThreadMarker::new() {
            let ns_app = NSApplication::sharedApplication(mtm);
            if ns_app.respondsToSelector(sel!(yieldActivationToApplication:)) {
                ns_app.yieldActivationToApplication(&target);
            }
        }
        target.activateWithOptions(NSApplicationActivationOptions::empty());
    }

    /// Brings our app forward so the entry box receives the keyboard.
    #[allow(deprecated)]
    pub fn activate_self() {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let ns_app = NSApplication::sharedApplication(mtm);
        if ns_app.respondsToSelector(sel!(activate)) {
            ns_app.activate();
        } else {
            ns_app.activateIgnoringOtherApps(true);
        }
    }

    /// Lets the window appear on every Space, including over full-screen apps.
    pub fn float_everywhere(window: &WebviewWindow) {
        let Ok(ptr) = window.ns_window() else { return };
        let ns_window: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
        ns_window.setCollectionBehavior(
            ns_window.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    // On Windows a non-focusable window never activates, and hiding the entry box
    // hands focus back to the previous window on its own.
    use super::WebviewWindow;

    pub fn remember_frontmost() {}
    pub fn restore_frontmost() {}
    pub fn activate_self() {}
    pub fn float_everywhere(_window: &WebviewWindow) {}
}

pub use imp::{activate_self, float_everywhere, remember_frontmost, restore_frontmost};

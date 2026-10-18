//! The webview → core commands (ARCHITECTURE §3.5). A secret crosses exactly once, in
//! `paste_secret`, and goes straight into the runner's stash (law 13).

use serde_json::Value;
use shroom_core::event::{Event, Field, Key, SecretField, Surface};
use shroom_core::scene::Action;
use tauri::State;

use crate::runtime::Runtime;

#[tauri::command]
pub fn act(rt: State<Runtime>, surface: Surface, action: Action, rev: Option<u64>) {
    if arms(surface, Some(&action), None) {
        rt.post(Event::Os(crate::runtime::os::grants()));
    }
    rt.post(Event::Act { surface, action, rev: rev.unwrap_or(0) });
}

#[tauri::command]
pub fn key(rt: State<Runtime>, surface: Surface, key: Key) {
    if arms(surface, None, Some(key)) {
        rt.post(Event::Os(crate::runtime::os::grants()));
    }
    rt.post(Event::Key { surface, key });
}

/// The arm-time guard's re-probe (Region 3: “every record arm re-probes grants”): Start,
/// or Return in the popover (its Return is Start at `config`), carries the grants as macOS
/// has them now, posted just before it — so the core arms on the truth, and replays the
/// beat a reset left missing.
pub fn arms(surface: Surface, action: Option<&Action>, key: Option<Key>) -> bool {
    surface == Surface::Popover && (matches!(action, Some(Action::Start)) || key == Some(Key::Return))
}

/// A paste into a secret field. Returns nothing: the core answers with the paste's shape
/// through the Scene, never with its text.
#[tauri::command]
pub fn paste_secret(rt: State<Runtime>, field: SecretField, text: String) {
    rt.paste_secret(field, text);
}

/// A webview's own error (a script error, an unhandled rejection), into the app's log: a
/// window that stops drawing says why. The message only, cut short.
#[tauri::command]
pub fn webview_error(rt: State<Runtime>, surface: String, message: String) {
    let message: String = message.chars().take(500).collect();
    rt.log(&format!("webview {surface}: {message}"));
}

/// A non-secret field's contents (the link name).
#[tauri::command]
pub fn field(rt: State<Runtime>, field: Field, value: String) {
    rt.field(field, value);
}

/// The first key the person typed into a popover that opened by itself (they clicked into
/// it, so it's key now): it becomes asked (law 5's unasked popover, N39).
#[tauri::command]
pub fn popover_keyed(rt: State<Runtime>) {
    rt.post(Event::PopoverKeyed);
}

#[tauri::command]
pub fn dismiss(rt: State<Runtime>) {
    rt.post(Event::PopoverDismissed);
}

/// The current slice for a surface, from the snapshot (no round-trip to the core).
#[tauri::command]
pub fn scene_get(rt: State<Runtime>, surface: Surface) -> Value {
    let snap = rt.snapshot.lock().unwrap();
    let Some(s) = snap.as_ref() else { return Value::Null };
    match surface {
        Surface::Popover => serde_json::to_value(&s.popover).unwrap_or(Value::Null),
        Surface::Countdown => serde_json::to_value(&s.countdown).unwrap_or(Value::Null),
        Surface::Label => serde_json::to_value(&s.label).unwrap_or(Value::Null),
        Surface::Onboarding => serde_json::to_value(&s.onboarding).unwrap_or(Value::Null),
        Surface::Library => serde_json::to_value(&s.library).unwrap_or(Value::Null),
        Surface::Settings => serde_json::to_value(&s.settings).unwrap_or(Value::Null),
        Surface::Feedback => serde_json::to_value(&s.feedback).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// The popover fits its content's height.
#[tauri::command]
pub fn popover_fit(app: tauri::AppHandle, height: f64) {
    crate::runtime::presenter::fit_popover(&app, height);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_and_the_popovers_return_carry_fresh_grants() {
        assert!(arms(Surface::Popover, Some(&Action::Start), None));
        assert!(arms(Surface::Popover, None, Some(Key::Return)));
        assert!(!arms(Surface::Popover, Some(&Action::Stop), None));
        assert!(!arms(Surface::Popover, None, Some(Key::Esc)));
        assert!(!arms(Surface::Onboarding, None, Some(Key::Return)), "the chapter's own Return asks macOS itself");
    }
}

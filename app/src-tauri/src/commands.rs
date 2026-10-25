//! The webview → core commands (ARCHITECTURE §3.5). A secret crosses exactly once, in
//! `paste_secret`, and goes straight into the runner's stash (law 13).

use serde_json::Value;
use shroom_core::event::{EditStep, Event, Field, Key, SecretField, Surface};
use shroom_core::scene::Action;
use tauri::{State, Webview};

use crate::runtime::Runtime;
use crate::runtime::presenter::editor_of;

/// A surface as its webview names it (`app/src/lib/ipc.ts`'s `Surface`). An editor window
/// names itself “editor”: which one it is, is its window's label's (`editor-<its number>`,
/// N73), never what its webview says — and a webview that isn't an editor window's acts on
/// no editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Named {
    Tray,
    Popover,
    Onboarding,
    Library,
    Settings,
    Feedback,
    Editor,
    Countdown,
    Bubble,
    Label,
    Toast,
}

/// The surface `named`, said by the webview labelled `label`.
pub fn surface(named: Named, label: &str) -> Option<Surface> {
    Some(match named {
        Named::Tray => Surface::Tray,
        Named::Popover => Surface::Popover,
        Named::Onboarding => Surface::Onboarding,
        Named::Library => Surface::Library,
        Named::Settings => Surface::Settings,
        Named::Feedback => Surface::Feedback,
        Named::Editor => Surface::Editor(editor_of(label)?),
        Named::Countdown => Surface::Countdown,
        Named::Bubble => Surface::Bubble,
        Named::Label => Surface::Label,
        Named::Toast => Surface::Toast,
    })
}

#[tauri::command]
pub fn act(rt: State<Runtime>, webview: Webview, surface: Named, action: Action, rev: Option<u64>) {
    let Some(surface) = self::surface(surface, webview.label()) else { return };
    if arms(surface, Some(&action), None) {
        rt.post(Event::Os(crate::runtime::os::grants()));
    }
    rt.post(Event::Act { surface, action, rev: rev.unwrap_or(0) });
}

#[tauri::command]
pub fn key(rt: State<Runtime>, webview: Webview, surface: Named, key: Key) {
    let Some(surface) = self::surface(surface, webview.label()) else { return };
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

/// The current slice for a surface, from the snapshot (no round-trip to the core) — an
/// editor window's, its own (N73).
#[tauri::command]
pub fn scene_get(rt: State<Runtime>, webview: Webview, surface: Named) -> Value {
    let Some(surface) = self::surface(surface, webview.label()) else { return Value::Null };
    let snap = rt.snapshot.lock().unwrap();
    let Some(s) = snap.as_ref() else { return Value::Null };
    match surface {
        Surface::Popover => serde_json::to_value(&s.popover).unwrap_or(Value::Null),
        Surface::Countdown => serde_json::to_value(&s.countdown).unwrap_or(Value::Null),
        Surface::Label => serde_json::to_value(&s.label).unwrap_or(Value::Null),
        Surface::Toast => serde_json::to_value(&s.toast).unwrap_or(Value::Null),
        Surface::Onboarding => serde_json::to_value(&s.onboarding).unwrap_or(Value::Null),
        Surface::Library => serde_json::to_value(&s.library).unwrap_or(Value::Null),
        Surface::Settings => serde_json::to_value(&s.settings).unwrap_or(Value::Null),
        Surface::Feedback => serde_json::to_value(&s.feedback).unwrap_or(Value::Null),
        Surface::Editor(id) => serde_json::to_value(s.editors.get(&id)).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// A step in an editor window (N63) — its own editor's, by its label (N73): `{"op":
/// "undo"}`, `{"op": "redo"}`, or one of the edit list's operations (`trim`, `cut`,
/// `put-back`, `apply`, `revert` — `shroom_edit::Op`), times in seconds on the recording.
/// The core takes it or refuses it; a step that doesn't read, or from a webview that isn't
/// an editor window's, is dropped here, and the next slice says what the edit is.
#[tauri::command]
pub fn edit(rt: State<Runtime>, webview: Webview, step: Value) {
    let Some(id) = editor_of(webview.label()) else { return rt.log("editor: a step from a window that isn't an editor's — dropped") };
    match edit_step(step) {
        Some(step) => rt.post(Event::Edit(id, step)),
        None => rt.log("editor: a step that doesn't read — dropped"),
    }
}

pub fn edit_step(step: Value) -> Option<EditStep> {
    match step.get("op").and_then(Value::as_str) {
        Some("undo") => Some(EditStep::Undo),
        Some("redo") => Some(EditStep::Redo),
        _ => serde_json::from_value::<shroom_edit::Op>(step).ok().filter(finite).map(EditStep::Op),
    }
}

/// Every time a step carries is a number the list can hold (JSON has no NaN, but a huge
/// literal reads as infinity).
fn finite(op: &shroom_edit::Op) -> bool {
    use shroom_edit::Op;
    match op {
        Op::Trim { in_, out } => in_.is_finite() && out.is_finite(),
        Op::Cut { start, end } | Op::Resize { start, end, .. } => start.is_finite() && end.is_finite(),
        Op::PutBack { .. } | Op::Apply { .. } | Op::Revert => true,
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

    /// The names the webviews use (`app/src/lib/ipc.ts`), and the editor windows told apart
    /// by their labels, never by what they say (N73).
    #[test]
    fn a_surface_is_its_webviews_name_and_an_editors_its_labels() {
        use shroom_core::world::EditorId;
        let named = |s: &str| serde_json::from_value::<Named>(serde_json::json!(s)).unwrap();
        assert_eq!(surface(named("popover"), "popover"), Some(Surface::Popover));
        assert_eq!(surface(named("library"), "library"), Some(Surface::Library));
        assert_eq!(surface(named("editor"), "editor-3"), Some(Surface::Editor(EditorId(3))));
        assert_eq!(surface(named("editor"), "editor-12"), Some(Surface::Editor(EditorId(12))));
        for not_one in ["library", "editor", "editor-", "editor-03", "editor-x", "editor-1-2", "editors-1"] {
            assert_eq!(surface(named("editor"), not_one), None, "{not_one}");
        }
        assert!(serde_json::from_value::<Named>(serde_json::json!({ "editor": 1 })).is_err(), "a webview never names an editor window");
    }

    /// Every surface is its own window's: a webview acts on, keys into and reads the slice
    /// of only the surface its window's label is — never another window's.
    #[test]
    fn a_surface_is_only_its_own_windows() {
        use crate::runtime::presenter::{COUNTDOWN, FEEDBACK, LABEL, LIBRARY, ONBOARDING, POPOVER, SETTINGS, TOAST};
        let named = |s: &str| serde_json::from_value::<Named>(serde_json::json!(s)).unwrap();
        let own = [("popover", POPOVER), ("onboarding", ONBOARDING), ("library", LIBRARY), ("settings", SETTINGS), ("feedback", FEEDBACK), ("countdown", COUNTDOWN), ("label", LABEL), ("toast", TOAST)];
        let labels = ["popover", "onboarding", "library", "settings", "feedback", "countdown", "label", "toast", "bubble", "main", "pattern", "editor-1", "tray"];
        for (name, label) in own {
            assert!(surface(named(name), label).is_some(), "{name} from its own window");
            for other in labels.iter().filter(|l| **l != label) {
                assert_eq!(surface(named(name), other), None, "{name} from {other}");
            }
        }
        assert_eq!(surface(named("bubble"), "bubble"), Some(Surface::Bubble));
        for other in ["popover", "settings", "main", "editor-1"] {
            assert_eq!(surface(named("tray"), other), None, "the tray has no webview: {other}");
            assert_eq!(surface(named("bubble"), other), None, "{other}");
        }
        assert_eq!(surface(named("editor"), "settings"), None);
    }

    /// A secret is pasted only in the windows with a paste field (the setup and Settings), and
    /// a typed field comes only from the windows that have one.
    #[test]
    fn pastes_and_fields_come_only_from_the_windows_that_have_them() {
        for label in ["onboarding", "settings"] {
            assert!(pastes_from(label), "{label}");
        }
        for label in ["popover", "library", "feedback", "countdown", "label", "toast", "bubble", "main", "pattern", "editor-1", ""] {
            assert!(!pastes_from(label), "{label}");
        }
        for label in ["onboarding", "settings", "library", "feedback"] {
            assert!(types_from(label), "{label}");
        }
        for label in ["popover", "countdown", "label", "toast", "bubble", "main", "pattern", "editor-1", ""] {
            assert!(!types_from(label), "{label}");
        }
    }

    #[test]
    fn start_and_the_popovers_return_carry_fresh_grants() {
        assert!(arms(Surface::Popover, Some(&Action::Start), None));
        assert!(arms(Surface::Popover, None, Some(Key::Return)));
        assert!(!arms(Surface::Popover, Some(&Action::Stop), None));
        assert!(!arms(Surface::Popover, None, Some(Key::Esc)));
        assert!(!arms(Surface::Onboarding, None, Some(Key::Return)), "the chapter's own Return asks macOS itself");
    }

    /// The editor's steps as its webview sends them (app/src/lib/ipc.ts `EditStep`).
    #[test]
    fn the_editors_steps_read_as_the_core_takes_them() {
        use serde_json::json;
        use shroom_edit::Op;
        assert_eq!(edit_step(json!({"op": "undo"})), Some(EditStep::Undo));
        assert_eq!(edit_step(json!({"op": "redo"})), Some(EditStep::Redo));
        assert_eq!(edit_step(json!({"op": "trim", "in": 8, "out": 241.5})), Some(EditStep::Op(Op::Trim { in_: 8.0, out: 241.5 })));
        assert_eq!(edit_step(json!({"op": "cut", "start": 150, "end": 162})), Some(EditStep::Op(Op::Cut { start: 150.0, end: 162.0 })));
        assert_eq!(edit_step(json!({"op": "put-back", "index": 0})), Some(EditStep::Op(Op::PutBack { index: 0 })));
        assert_eq!(edit_step(json!({"op": "apply", "index": 1})), Some(EditStep::Op(Op::Apply { index: 1 })));
        assert_eq!(edit_step(json!({"op": "revert"})), Some(EditStep::Op(Op::Revert)));
        for bad in [json!({"op": "cut", "start": "x", "end": 2}), json!({"op": "cut", "start": f64::INFINITY, "end": 2}), json!({"op": "nope"}), json!({}), json!("undo"), json!({"op": "put-back", "index": -1})] {
            assert_eq!(edit_step(bad.clone()), None, "{bad}");
        }
    }
}

"""The rules: moves that hold for a whole group of states, written once (Bagrat's pick (b),
DECISIONS B133). Each is the spec line the enumeration tests already lean on
(crates/shroom-core/tests/enumerate.rs's `by_rule` and `pop_rule`), as data. The tests
expand a rule into its moves (phase 2). Imported by assemble.py.

A rule: `from` and `to` are state ids or `@group`s (`*` is any state); `on` is a list of
event patterns (EVENTS); `reverse` also allows `to → from`; `family` holds only between a
state and its own faces.
"""

EVENTS = {
    "about": "How a rule names the events it moves on. A pattern is an event's kind as the core's `Event` names it: `TrayClicked`, `WindowClosed`, `Act(Restart)`, `Cloud(Swept)`, `Failed(Wire)`; a bare `Cloud`, `Rec`, `Os` or `Failed` matches every kind under it. The words below stand for several.",
    "answer": ["Cloud", "Rec", "Failed", "Tick", "Os", "Member", "Upload", "Join", "Scanned"],
    "away": ["Act(Dismiss)", "Key(Esc)", "PopoverDismissed"],
    "click": ["TrayClicked"],
    "any": ["*"],
}

GROUPS = {
    "network-bound": {"title": "the wizard's network-bound steps", "states": ["token", "card", "gate-email", "gate-card", "name", "name-taken", "wiring", "space-found", "repair-mood", "token-repair-mood", "re-entry-as-settings"], "cites": ["N18"]},
    "token-row": {"title": "the token row and its faces", "states": ["token", "token-repair-mood", "token-garbled", "token-wrong-powers", "token-other-account", "token-pick-account"], "cites": ["N14"]},
    "wizard-rows": {"title": "the plan's rows, account to done", "states": ["account", "account-signing-up", "token", "token-garbled", "token-wrong-powers", "token-pick-account", "card", "gate-email", "gate-card", "name", "name-taken", "wiring", "space-found", "done"], "cites": ["N13"]},
    "chapter": {"title": "the permissions chapter (row six)", "states": ["done", "perm-mic", "perm-screen", "relaunching", "perm-priming", "all-set"], "cites": ["N17"]},
    "repair": {"title": "Settings · Space's re-probe and the rows it can land on", "states": ["re-entry-as-settings", "repair-mood", "token-repair-mood", "token", "token-garbled", "token-wrong-powers", "token-other-account", "gate-email", "gate-card", "name-changed", "wiring", "space-storage-gone", "perm-screen", "relaunching", "settings-space-no-token", "settings-space-member"], "cites": ["N42"]},
    "popovers": {"title": "the tray's popovers", "states": ["config", "paused", "discard-confirm", "restart-confirm", "quit-confirm", "stopped-offline", "stopped-stranded", "permission-lost", "disk-full-stop"], "cites": ["N21"]},
    "paused-faces": {"title": "paused and its auto-pause faces", "states": ["paused", "display-lost", "mic-lost", "camera-lost", "mac-away", "mic-permission-lost"], "cites": ["N24", "N39"]},
    "auto-pause": {"title": "the auto-pause faces (the map's “mic · camera · sleep · lock”)", "states": ["mac-away", "mic-lost", "camera-lost", "mic-permission-lost"], "cites": ["N39"]},
    "stop-popovers": {"title": "a stop's own popovers", "states": ["stopped-offline", "stopped-stranded", "permission-lost", "disk-full-stop"], "cites": ["N34", "N39", "N40"]},
    "idle": {"title": "the tray at rest", "states": ["idle-ready", "idle-unprovisioned"], "cites": ["law 5"]},
    "space-tab": {"title": "the Space tab, every mood and the Region 2 state beneath it", "states": ["settings-space", "settings-space-inspection", "settings-space-repair", "settings-space-no-token", "settings-space-member", "re-entry-as-settings", "repair-mood", "token-repair-mood", "token", "token-garbled", "token-wrong-powers", "token-other-account", "gate-email", "gate-card", "name-changed", "wiring", "wizard-offline", "wizard-unexpected", "space-storage-gone", "perm-screen", "relaunching"], "cites": ["N36", "N42"]},
    "exit-sheets": {"title": "the Space tab's exit sheets", "states": ["disconnect-confirm", "disconnect-confirm-no-token", "leave-confirm"], "cites": ["N36"]},
    "settings-tabs": {"title": "the Settings window's tabs", "states": ["settings-general", "settings-members", "invite-born", "add-another-mac"], "cites": ["N36"]},
    "join-doors": {"title": "where an invite can arrive", "states": ["idle-ready", "idle-unprovisioned", "config", "welcome", "first-launch", "join-waiting", "join-expired", "join-used", "join-space-missing"], "cites": ["N30", "N31"]},
    "join-arrival": {"title": "the invite window's faces on arrival", "states": ["member-welcome", "join-waiting", "join-expired", "join-used", "join-space-missing", "join-already-here", "join-switch-confirm", "join-switch-confirm-uploads", "join-switch-confirm-saved-take", "joining"], "cites": ["N30", "N31", "#20"]},
    "banners": {"title": "config's banners (one at a time, N38)", "states": [], "cites": ["N38"]},
}

RULES = [
    {"id": "offline", "label": "offline, then back where the facts stand", "from": ["@network-bound"], "to": ["wizard-offline"], "on": ["any"], "reverse": True, "kind": "fail",
     "says": "wizard-offline interposes on any network-bound step and resumes where the facts stand.", "cites": ["N18", "N42"], "status": "ratified"},
    {"id": "offline-answer", "label": "a call answering proves the network is back", "from": ["wizard-offline"], "to": ["*"], "on": ["Cloud"], "kind": "auto",
     "says": "A wizard call that was out when the network dropped answers after all: the face re-derives from its answer.", "cites": ["ARCHITECTURE §3.3", "B134"], "status": "ratified"},
    {"id": "unexpected", "label": "an unclassified failure, and back", "from": ["*"], "to": ["wizard-unexpected"], "on": ["any"], "reverse": True, "kind": "fail",
     "says": "The one face for a failure nothing else classifies, from any wizard face, and back when an answer clears it or Try again.", "cites": ["#25", "B134"], "status": "ratified"},
    {"id": "close-window", "label": "✕ closes the window", "from": ["*"], "to": ["@idle", "closed", "config"], "on": ["WindowClosed", "Act(CloseWindow)"], "kind": "side",
     "says": "✕ closes the window at any row — Later's twin: nothing is lost, and the next door re-derives it.", "cites": ["N19", "N21"], "status": "ratified"},
    {"id": "token-paste", "label": "every paste meets the shape check, then verify", "from": ["@token-row"], "to": ["@token-row"], "on": ["SecretPasted(Token)", "answer"], "kind": "auto",
     "says": "Any new paste starts the token row over: the shape check first, then verify; the row's spinner while a paste is checked or saved (the Checking beat).", "cites": ["N14", "ARCHITECTURE §3.7"], "status": "ratified"},
    {"id": "rows-mark-themselves", "label": "the rows mark themselves from the facts", "from": ["@wizard-rows"], "to": ["@wizard-rows"], "on": ["answer"], "kind": "auto",
     "says": "An answer can tick several rows: the plan's rows mark themselves from the facts each answer brings (token → gates → bucket → page → name).", "cites": ["N13", "law 1"], "status": "ratified"},
    {"id": "chapter-derived", "label": "the chapter's step follows the grants", "from": ["@chapter"], "to": ["@chapter"], "on": ["answer"], "kind": "auto",
     "says": "Row six's step is derived from the grants and the warm-up, never stored: a grant already given skips its step, one switched off steps back.", "cites": ["N17", "law 1"], "status": "ratified"},
    {"id": "turn-back-on", "label": "Turn it back on", "from": ["*"], "to": ["perm-screen"], "on": ["Act(TurnBackOn)"], "kind": "flow",
     "says": "Row six is live on every Mac: Turn it back on runs the chapter's screen step wherever recording-off shows.", "cites": ["N36", "B134"], "status": "ratified"},
    {"id": "space-reprobe", "label": "the Space tab re-probes every row", "from": ["@repair"], "to": ["@repair"], "on": ["answer", "WindowFocused"], "kind": "auto",
     "says": "Settings · Space re-probes every row on entry; the first broken row is current with Region 2's state beneath it, or every row ticks (the inspection).", "cites": ["N42", "B134"], "status": "ratified"},
    {"id": "popover-away", "label": "click-away or Esc closes it", "from": ["@popovers"], "to": ["@idle", "closed", "paused", "recording"], "on": ["away"], "kind": "side",
     "says": "Any popover closes on click-away or Esc; nothing is lost, and the tray stands where the facts are.", "cites": ["law 9", "N21"], "status": "ratified"},
    {"id": "paused-faces", "label": "paused shows its cause", "from": ["@paused-faces"], "to": ["@paused-faces"], "on": ["answer", "click"], "kind": "auto",
     "says": "The auto-pause faces are faces of paused, re-derived from the cause and the Mac's presence; the tray click on the pause bars opens the popover at its cause; a Mac away shows nothing until it's back.", "cites": ["N24", "N39"], "status": "ratified"},
    {"id": "quit-asks", "label": "Quit with uploads draining asks first", "from": ["@idle", "closed", "config", "discard-confirm", "quit-confirm", "stopped-offline", "stopped-stranded", "permission-lost", "disk-full-stop"], "to": ["quit-confirm"], "on": ["QuitRequested"], "kind": "side",
     "says": "Quit shroom — the right-click menu or the popover's — while uploads drain asks first, over whatever idle popover stands.", "cites": ["N21"], "status": "ratified"},
    {"id": "take-stops-itself", "label": "the take finalizes itself", "from": ["@paused-faces", "recording", "discard-confirm", "restart-confirm"], "to": ["stopping", "permission-lost", "disk-full-stop"], "on": ["answer"], "kind": "fail",
     "says": "Mid-take — a paused or asking take too — the screen grant revoked or the disk at its floor finalizes the take: the stop beat, then its own popover.", "cites": ["N39", "#6"], "status": "ratified"},
    {"id": "stop-popover-holds", "label": "a stop's popover holds until dismissed or answered", "from": ["@stop-popovers"], "to": ["config", "@idle", "closed"], "on": ["any"], "kind": "side",
     "says": "A stop's popover is the stop's own: it holds until dismissed or answered (back online, the page opened); the next door is config.", "cites": ["N34", "N39", "N40", "B134"], "status": "ratified"},
    {"id": "tray-door", "label": "the tray click is the door", "from": ["@idle", "closed"], "to": ["config", "@idle", "closed", "@wizard-rows", "@chapter", "welcome"], "on": ["click"], "kind": "flow",
     "says": "The tray click is the door: idle → config, or the setup window at the row the facts derive (a Mac whose chapter isn't done opens at row six).", "cites": ["law 5", "N21", "#5"], "status": "ratified"},
    {"id": "relaunch", "label": "a relaunch re-derives where you were", "from": ["*"], "to": ["*"], "on": ["Booted", "Reopened"], "kind": "auto",
     "says": "Every entry re-probes: a launch or a relaunch lands on derive(probes), never on a remembered step.", "cites": ["law 1"], "status": "ratified"},
    {"id": "exit-sheet", "label": "Disconnect… or Leave…", "from": ["@space-tab", "config"], "to": ["@exit-sheets"], "on": ["Act(Disconnect)"], "kind": "side",
     "says": "The Space tab's exit is there in every mood: Disconnect… (an owner-class Mac) or Leave… (a member) opens its sheet — from a banner's button too (a removed Mac, a member's).", "cites": ["N36", "B134"], "status": "ratified"},
    {"id": "exit-sheet-back", "label": "Back keeps the space", "from": ["@exit-sheets"], "to": ["@space-tab"], "on": ["Act(Back)", "Key(Esc)", "away"], "kind": "side",
     "says": "Back, or Esc, closes the sheet; the Space tab re-probes where it was.", "cites": ["N36", "law 9", "B134"], "status": "ratified"},
    {"id": "settings-door", "label": "Settings opens where it should", "from": ["config", "stopped-stranded"], "to": ["@settings-tabs", "@space-tab"], "on": ["Act(OpenSettings)", "Act(ReconnectSpace)", "Act(AddToken)"], "kind": "flow",
     "says": "The gear opens Settings on General the first time, then the tab last used; Reconnect your space and Add a token here open the Space tab, which re-probes and lands on the row the facts derive.", "cites": ["N36", "N42", "law 1", "B134"], "status": "ratified"},
    {"id": "settings-tabs", "label": "the tabs", "from": ["@settings-tabs", "@space-tab"], "to": ["@settings-tabs", "@space-tab"], "on": ["Act(SettingsTab)"], "kind": "flow",
     "says": "Clicking a tab is an entry: General, Space (which re-probes, law 1) and Members (owner-class only).", "cites": ["N36", "law 1", "B134"], "status": "ratified"},
    {"id": "invite-arrives", "label": "an invite opens its window", "from": ["@join-doors"], "to": ["@join-arrival"], "on": ["Member", "Failed(Peek)", "SecretPasted(Invite)", "click", "Scanned"], "kind": "flow",
     "says": "An invite that arrives — the deep link, a paste, the installer's file, or one remembered at the tray click — opens the invite window at the face peek's answer derives: the welcome, waiting, a dead link's line, already here, or the switch's question.", "cites": ["N30", "N31", "#2", "#20", "B134"], "status": "ratified"},
    {"id": "faces", "label": "a face comes and goes with its cause", "from": ["*"], "to": ["*"], "on": ["any"], "family": True, "kind": "auto",
     "says": "Faces are render properties of their state: a state and its faces, and one state's faces, re-derive into one another as their causes come and go.", "cites": ["N18", "law 1", "N34"], "status": "ratified"},
    {"id": "banners", "label": "one banner at a time, the highest", "from": ["@banners", "none"], "to": ["@banners", "none"], "on": ["any"], "kind": "auto",
     "says": "config's banner shows while its cause stands and clears by itself when it goes; a higher banner hides a lower one (one at a time).", "cites": ["N38", "law 1"], "status": "ratified"},
]


def _on(pattern_list, on):
    for p in pattern_list:
        for q in EVENTS.get(p, [p]) if p in EVENTS and p != "about" else [p]:
            if q == "*" or on == q or on.startswith(q + "("):
                return True
    return False


def _in(spec, s, expand):
    return "*" in spec or s in expand(spec)


def family(f, t, ancestors):
    """A state and its own face, or two faces of one state."""
    af, at = ancestors(f), ancestors(t)
    return f in at or t in af or (len(af) > 1 and len(at) > 1 and af[1] == at[1])


def matches(r, f, t, on, expand, ancestors):
    if not _on(r["on"], on):
        return False
    if r.get("family"):
        return family(f, t, ancestors)
    if _in(r["from"], f, expand) and _in(r["to"], t, expand):
        return True
    return bool(r.get("reverse")) and _in(r["to"], f, expand) and _in(r["from"], t, expand)

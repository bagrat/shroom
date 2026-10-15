"""The implementation, laid over the old spec (machine-json phase 1): the states only the code
has, the copy that shipped under a logged deviation, and the build's own lines (DECISIONS'
copy table). Every change is `provisional` unless Bagrat already ratified it, and cites its
row. Imported by assemble.py, which records each change for the report.
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

REGION_TITLES = {
    1: "acquisition → first launch",
    2: "the cloud setup wizard",
    3: "the permissions chapter and the tray",
    4: "stop → the link → the recipient",
    5: "the tail: library, settings, updates",
    6: "members and the join machine",
}

# The states only the code has.
CODE_STATES = {
    "wizard-unexpected": {
        "after": "wizard-offline",
        "region": 2, "kind": "failure-state", "parent": None, "surface": "window", "status": "provisional",
        "summary": "The wizard's one face for a failure nothing else classifies: the call a wizard face waits on (the paste's check, the token's save, the probe pass, wiring; Settings · Space's repair mood the same) fails Refused, Unclassified, or panicked, and no answer has cleared it. The spinner rests under the current row with one amber line and Try again; the backoff keeps retrying meanwhile. Under the name row the line sits beneath the field and Claim stays live.",
        "entry": "A wizard call failed in a way nothing else classifies, and no answer has cleared it. Cloudflare busy (429, 5xx) and wiring's propagation wait running out are timeouts — a calm retry, never this face.",
        "said": [
            {"text": "Something unexpected got in the way — everything so far is saved. I'll keep trying.", "role": "line", "copy": "provisional", "note": "amber, under the current row"},
            {"text": "Try again", "role": "button", "copy": "provisional"},
        ],
        "actions": [{"label": "Try again", "kind": "secondary", "return_default": False, "note": "every wizard call asked again now, the window re-probed"}],
        "cites": ["#25"],
    },
    "restart-confirm": {
        "after": "discard-confirm",
        "region": 3, "kind": "state", "parent": None, "surface": "popover", "status": "provisional",
        "summary": "Restart's question, in the paused popover: Restart replaced the take with one click, one row from Stop. Its Restart starts the count, which stays cancellable and is still Restart's undo: a cancelled count gives the old take back, paused. The Discard question's Restart instead is an answer already and starts the count at once.",
        "entry": "Restart on the paused popover, any face of it.",
        "said": [
            {"text": "Start over?", "role": "title", "copy": "provisional"},
            {"text": "This replaces the 4:32 you've recorded so far.", "role": "line", "copy": "provisional", "note": "the length varies"},
            {"text": "Back", "role": "button", "copy": "provisional"},
            {"text": "Restart", "role": "button", "copy": "provisional", "note": "red outline, rightmost"},
        ],
        "actions": [
            {"label": "Back", "kind": "secondary", "return_default": False, "goes_to": "paused", "note": "Esc is Back"},
            {"label": "Restart", "kind": "destructive", "return_default": False, "goes_to": "countdown", "note": "no Return default (law 9)"},
        ],
        "cites": ["#44", "N24", "N39"],
    },
    "owner-mac-removed": {
        "after": "space-gone",
        "region": 6, "kind": "banner", "parent": "config", "surface": "banner", "status": "provisional",
        "summary": "A removed owner-class Mac (its /me or mint answers 401, or 403 removed): config's banner under the amber dot, Start greyed, a stop stranded. N34's three faces are member faces; this is the owner's own Mac.",
        "entry": "An owner-class Mac's /me or mint answers that it was removed from the space.",
        "said": [
            {"text": "This Mac was removed from your space. Recordings on this Mac are safe — add it again from Members on your other Mac.", "role": "line", "copy": "provisional"},
            {"text": "Save to Movies", "role": "button", "copy": "provisional", "note": "while takes wait"},
            {"text": "Disconnect this Mac…", "role": "button", "copy": "provisional"},
        ],
        "actions": [
            {"label": "Save to Movies", "kind": "secondary", "return_default": False},
            {"label": "Disconnect this Mac…", "kind": "secondary", "return_default": False, "goes_to": "disconnect-confirm-no-token"},
            {"label": "Start recording", "kind": "filled-default", "return_default": False, "note": "Greyed: a new video would have nowhere to go."},
        ],
        "cites": ["#21"],
    },
}

# States the build retired, by Bagrat's call.
RETIRED = {
    "guided-agent-overlay": ("#50", "B132", "the BYO-agent skill is retired; shroom.am/agent is the agent's door — retired by Bagrat, 2026-10-14"),
}

# Said-lines whose shipped words differ, under a logged row. (state, old) -> (new or None, cites, ratified?)
COPY = [
    ("landing", "For macOS 14 or newer, Apple silicon and Intel.", "For macOS 14 or newer, Apple silicon.", ["B7", "B44", "copy table"], "the shape is ratified with its facts to fill; everything built is arm64-only — ratified by Bagrat (B134)"),
    ("join-landing", "For macOS 14 or newer, Apple silicon and Intel.", "For macOS 14 or newer, Apple silicon.", ["B7", "B44", "copy table"], "the same line on the join page — ratified by Bagrat (B134)"),
    ("config", "Camera", "Camera only", ["#43"], "ratified by Bagrat in the asking (2026-10-13)"),
    ("paused", "no Quit while a take is open", None, ["#45"], "ratified by Bagrat in the drawing (the note goes)"),
    ("fully-live", "Fully live ✓", None, ["#46"], "ratified by Bagrat in the drawing: the page's upload line and its bar just go when the last file is up; the popover still says “Fully live ✓”"),
    ("invite-born", "v.myshroom.workers.dev/join/7xk2pm", "v.myshroom.workers.dev/join/", ["#22"], "the pill shows the address up to /join/, the code masked; Copy link is the only way to it — ratified by Bagrat (B134)"),
    ("add-another-mac", "v.myshroom.workers.dev/join/q8w3zt", "v.myshroom.workers.dev/join/", ["#22"], "the same pill — ratified by Bagrat (B134)"),
]

NOTES = [
    ("installing", "Getting shroom… verified ✓", "prints as “Getting shroom…”, curl's own bar, then “verified ✓” on the line after (DEVIATIONS #13); the words and their order are unchanged"),
]

# New lines the shipped app says, on states the spec has. (state, text, role, cites, note)
ADD = [
    ("config", "Camera off", "menu-item", ["#43"], "Show yourself's last pick; ratified by Bagrat in the asking"),
    ("config", "No mic", "menu-item", ["#43"], "the microphone's last pick; ratified by Bagrat in the asking"),
    ("config", "No microphone — your video will be silent.", "line", ["#43"], "no microphone at all"),
    ("paused", "Restart", "button", ["#44"], "now opens restart-confirm"),
]

# DECISIONS' “Copy written during the build” table: each row's wordings, on the state that
# says them (None: not a state's line — a file, a page outside the machine, or no wording).
COPY_TABLE_STATES = [  # (the row's “Where”, as it begins; the state)
    ('The plan, rows behind you', 'done'),
    ('The plan, the name row while current', 'name'),
    ('The plan, row six while current', 'perm-mic'),
    ('`space-found`, one video', 'space-found'),
    ('The installer, no build for this kind of Mac', 'install-unsupported-os'),
    ("The installer, an update the app won't close for", 'updating'),
    ("The installer, macOS won't open the copy it just placed (`in", 'installing'),
    ("The installer, this Mac can't take it (no room for the downl", 'installing'),
    ("The installer, run on a machine that isn't a Mac (a Linux bo", 'install-unsupported-os'),
    ("The installer, a release that's ours to fix", 'install-checksum-fail'),
    ("The installer, a signature that doesn't verify", 'install-checksum-fail'),
    ('The installer, somewhere restricted', 'installing'),
    ("The token step's pre-filled page", None),
    ('The Library, a row uploading from another Mac', 'library'),
    ('The Library footer, one video', 'library'),
    ("The Library's dates", 'library'),
    ('The Library delete sheet, ten or more', 'delete-confirm-several'),
    ('The Library, Discard… on a “Removed from the space” row', 'discard-confirm-saved-take'),
    ('The Disconnect sheet, one stranded take', 'disconnect-confirm'),
    ('The Disconnect sheet, one upload draining', 'disconnect-confirm'),
    ('The Disconnect sheet, a recovered take waiting', 'disconnect-confirm'),
    ("The join page, while the space's records can't be read (no r", 'join-landing'),
    ("The join page's owner view of an own-Mac link", 'join-landing-owner-view'),
    ('The join page, an own-Mac link already used (to anyone but t', 'join-used-page'),
    ('`join-space-missing` at arrival, no inviter known', 'join-space-missing'),
    ("Members, an expired invite's row", 'invite-born'),
    ("Members, another of the owner's Macs", 'invite-born'),
    ('`joining · own-Mac variant`, row one', 'joining-own-mac'),
    ('`join-switch-confirm`, one upload draining', 'join-switch-confirm-uploads'),
    ('The Leave sheet, uploads draining', 'leave-confirm'),
    ('The member banners, two takes waiting', 'member-revoked'),
    ("A removed owner-class Mac's banner", 'owner-mac-removed'),
    ('An own-Mac link past its hour, in the app', 'join-expired'),
    ('`join-waiting`, no inviter known (peek never got through)', 'join-waiting'),
    ("`join-switch-confirm`, takes the old space can't take", 'join-switch-confirm'),
    ("The popover's Start while a switch's join runs", 'config'),
    ('The invite window, a space whose page is behind', 'member-welcome'),
    ("The player's controls, named for screen readers and keyboard", 'live'),
    ("The restyled player's speed and sound controls, named the sa", 'live'),
    ('`disk-low`, one to nine minutes', 'disk-low'),
    ("Displays of one name that place can't tell apart", 'config'),
    ("Unfurls' og:description / twitter:description — **none writt", None),
    ("The wizard's one face for a failure nothing else classifies ", 'wizard-unexpected'),
    ('Save to Movies while its copy runs (the banners, the exit sh', 'stopped-stranded'),
    ("Save to Movies that couldn't make its file", 'stopped-stranded'),
    ('Settings · General, nothing known about a newer version (no ', 'settings-general'),
    ('`join-used` in the app, an own-Mac link', 'join-used'),
    ("`join-waiting`, an own-Mac link whose space isn't answering", 'join-waiting'),
    ("The Members face's composer line, after a link that didn't c", 'invite-born'),
    ("The Members face's composer line, after a Remove… that didn'", 'invite-born'),
    ("The landing's system-requirements line (`landing`, Install b", 'landing'),
    ("The landing's browser title and link-preview title", 'landing'),
    ("The landing's search description", 'landing'),
    ("The landing's footer", 'landing'),
    ("The agent page's own lines (`site/agent.md`: the line under ", None),
    ("The landing's agent door (`landing`, Install block, below th", 'landing'),
    ("FFmpeg's notice in the release bundle (`tools/ffmpeg-NOTICE.", None),
    ("The preview's build script, to Bagrat in Terminal (`tests/ch", None),
]

# Fixture transitions whose target was prose: the state(s) it names, or None (a derived
# return or the app quitting — the rules cover it).
TRANSITION_TARGETS = {
    "Region 4's player, live": ["live"],
    "the relaunched app, tray-only and silent": ["idle-ready"],
    "member-welcome · join-already-here · join-switch-confirm (Region 6)": ["member-welcome", "join-already-here", "join-switch-confirm"],
    "the tray click's door (N21)": [None],
    "wizard": ["account"],
    "(shroom quits)": [None],
    "(derived — the current row's state)": [None],
    "(derived — re-probe)": [None],
    "Settings window (Region 5)": ["settings-general"],
    "Region 4 — the link moment": ["owner-page"],
    "owner page (Region 4), owner-mode via the owner-keyed URL": ["owner-page"],
    "discard-confirm (N25's variant: “4:32 was saved. There's no undo.”)": ["discard-confirm-saved-take"],
    "library (that Mac's row reads 'Removed from the space')": ["library"],
    "settings · General / settings · Space / settings · Members": ["settings-general", "settings-space", "settings-members"],
    "settings · Space (row six current at perm-screen)": ["settings-space"],
    "perm-screen (the permissions chapter at the missing beat, in this window)": ["perm-screen"],
    "join-already-here (Region 6)": ["join-already-here"],
    "join-switch-confirm (Region 6, N31)": ["join-switch-confirm"],
    "token (the wizard; this Mac becomes a token-holder)": ["token"],
    "invite-born (Region 6)": ["invite-born"],
    "hidden under the higher banner (one banner at a time, N38)": [None],
    "config (carrying the update-ready banner)": ["config"],
    "plain tray face": [None],
    "that window reopened at its derived state after relaunch": [None],
    "config (no status line)": ["config"],
    "update-ready (implied by 'will try again later'; whether the banner and dot persist meanwhile is not stated)": ["update-ready"],
    "config (banner clears)": ["config"],
}


def copy_table():
    md = (ROOT / "docs/DECISIONS.md").read_text()
    table = md.split("## Copy written during the build")[1].split("\n## ")[0]
    rows = [l for l in table.splitlines() if l.startswith("| ") and not l.startswith("| Where") and not l.startswith("|---")]
    out = []
    for n, row in enumerate(rows, 1):
        cells = [c.strip() for c in row.strip("|").split(" | ")]
        out.append((n, cells[0], re.findall(r"“([^”]+)”", cells[1]), "Ratified by Bagrat" in row))
    return out


def apply(states, order, note, said_ids):
    for sid, st in CODE_STATES.items():
        st = dict(st)
        after = st.pop("after")
        st["board"] = None
        st["was"] = None
        states[sid] = st
        order.insert(order.index(after) + 1, sid)
        note("code-states", f"`{sid}` (Region {st['region']}, {st['kind']}) — only the code has it; cites {', '.join(st['cites'])}")
    for sid, (dev, dec, why) in RETIRED.items():
        del states[sid]
        order.remove(sid)
        note("retired", f"`{sid}` — {why} (DEVIATIONS {dev}, {dec})")
    for sid, old, new, cites, why in COPY:
        lines = states[sid]["said"]
        l = next(x for x in lines if x["text"] == old)
        if new is None:
            lines.remove(l)
            note("copy-dropped", (sid, old, cites, why, l["copy"]))
        else:
            l["text"] = new
            l["was"] = old
            l["cites"] = cites + ([RULING] if f"({RULING})" in why else [])
            note("copy-changed", (sid, old, new, cites, why, l["copy"]))
            l["copy"] = "ratified" if "ratified by Bagrat" in why else "provisional"
    for sid, text, why in NOTES:
        l = next(x for x in states[sid]["said"] if x["text"] == text)
        l["note"] = (l.get("note", "") + "; " if l.get("note") else "") + why
        note("copy-noted", (sid, text, why))
    for sid, text, role, cites, why in ADD:
        if any(x["text"] == text for x in states[sid]["said"]):
            continue
        states[sid]["said"].append({"text": text, "role": role, "copy": "ratified" if "ratified by Bagrat" in why else "provisional", "note": why, "cites": cites})
        note("copy-added", (sid, text, cites, why))
    for n, where, words, ratified in copy_table():
        hits = [st for key, st in COPY_TABLE_STATES if where.startswith(key)]
        assert len(hits) == 1, f"copy table row {n} ({where[:60]!r}) needs one entry in COPY_TABLE_STATES"
        sid = hits[0]
        if sid is None:
            note("copy-table-skipped", (n, where, words))
            continue
        assert sid in states, f"copy table row {n} → {sid}"
        for w in words:
            if " …" in w or "… " in w.rstrip("…"):
                continue  # the table's shorthand for a variant (“One video … once it's done.”), not a line
            if any(x["text"] == w for x in states[sid]["said"]):
                continue
            states[sid]["said"].append({"text": w, "role": "line", "copy": "ratified" if ratified else "provisional", "note": f"one case the spec didn't print, so it varies by case — the copy table: {where}", "cites": ["copy table"]})
            note("copy-table", (sid, w, where))


def trigger_words(on):
    fixed = {"WindowClosed": "✕ closes the window", "TrayClicked": "tray click", "PopoverDismissed": "click-away", "Tick": "a deadline passes",
             "Key(Return)": "Return", "Key(Esc)": "Esc", "Member": "the join lane answers"}
    if on in fixed:
        return fixed[on]
    m = re.match(r"(\w+)\((\w+)\)", on)
    if not m:
        return on
    words = re.sub(r"(?<!^)([A-Z])", r" \1", m.group(2)).lower()
    if m.group(1) == "Act":
        return words[0].upper() + words[1:]
    return {"Rec": "the recorder: ", "Cloud": "Cloudflare: ", "Os": "macOS: ", "Failed": "a call fails: "}.get(m.group(1), m.group(1) + ": ") + words


# The moves only the code makes, and the rows that put them there.
CITES = {
    ("discard-confirm-saved-take", "library"): ["#15"],
    ("join-switch-confirm", "idle-ready"): ["N31"], ("join-switch-confirm", "idle-unprovisioned"): ["N31"], ("join-switch-confirm", "paused"): ["N31"],
    ("joining", "all-set"): ["N33"], ("joining", "done"): ["N33"], ("joining", "join-already-here"): ["N31"],
    ("token-pick-account", "token"): ["N14", "ARCHITECTURE §3.7"],
    ("config", "discard-confirm"): ["N25"], ("disk-full-stop", "discard-confirm"): ["N25"], ("permission-lost", "discard-confirm"): ["N25"], ("discard-confirm", "config"): ["N25"],
    ("config", "stopping"): ["N25"], ("idle-ready", "stopping"): ["N35"],
    ("countdown", "idle-ready"): ["N39", "#6"],
    ("idle-ready", "idle-unprovisioned"): ["N36"],
    ("idle-ready", "recording"): ["N39"],
    ("idle-ready", "stopped-offline"): ["N40"], ("idle-ready", "stopped-stranded"): ["N40", "N34"],
    ("idle-unprovisioned", "idle-ready"): ["N13"],
    ("quit-confirm", "idle-ready"): ["law 9", "N21"],
    ("recording", "config"): ["#6"], ("recording", "stopping"): ["N39"],
    ("restart-confirm", "countdown"): ["#44"], ("restart-confirm", "paused"): ["#44"],
    ("token", "settings-space"): ["N33"], ("settings-general", "settings-space"): ["N36"], ("space-storage-gone", "wiring"): ["N42", "B52"],
    ("joining", "member-welcome"): ["#30"], ("paused", "restart-confirm"): ["#44"],
    ("stopping", "config"): ["#6"], ("stopping", "disk-full-stop"): ["N39"], ("stopping", "idle-ready"): ["N25"], ("stopping", "permission-lost"): ["N39"],
}


def cites_for(f, t, m, why):
    out = list(CITES.get((f, t), []))
    for w in why:
        for c in re.findall(r"DEVIATIONS #(\d+)|\b(N\d+|B\d+)\b|#(\d+)", w):
            x = "#" + c[0] if c[0] else (c[1] or "#" + c[2])
            if x not in out:
                out.append(x)
    return out


def kind_for(e, states):
    to = states[e["to"]]
    if to["kind"] in ("failure-state",) or "fail" in to["kind"]:
        return "fail"
    if "core-run" in e["source"] and not ({"map", "fixture"} & set(e["source"])):
        return "flow"
    return "flow"


# Bagrat's ruling on the phase-1 report (DECISIONS B134, 2026-10-14: items 1, 2, 3, 5 and 6 ratified).
RULING = "B134"
RULED_STATES = ["restart-confirm"]  # (1), with its words and its edges (every edge citing #44)
RULED_EDGES = [  # (5): the moves the code makes between ratified states that no drawing showed
    ("idle-unprovisioned", "idle-ready"), ("token-pick-account", "token"), ("idle-ready", "idle-unprovisioned"), ("idle-ready", "recording"),
    ("idle-ready", "stopping"), ("idle-ready", "stopped-offline"), ("idle-ready", "stopped-stranded"), ("config", "discard-confirm"),
    ("config", "stopping"), ("countdown", "idle-ready"), ("recording", "config"), ("recording", "stopping"), ("discard-confirm", "config"),
    ("discard-confirm-saved-take", "library"), ("permission-lost", "discard-confirm"), ("disk-full-stop", "discard-confirm"),
    ("quit-confirm", "idle-ready"), ("stopping", "idle-ready"), ("stopping", "config"), ("stopping", "permission-lost"),
    ("stopping", "disk-full-stop"), ("joining", "done"), ("joining", "all-set"), ("join-switch-confirm", "idle-unprovisioned"),
    ("join-switch-confirm", "idle-ready"), ("join-switch-confirm", "paused"),
]
# (6): the rules written from the build are ratified in rules.py itself.

# Edges the build changed, by its rows.
# Labels for edges only the core's arms name.
LABELS = {
    ("token", "settings-space"): "the token saved: this Mac holds it now (Add a token here)",
    ("settings-general", "settings-space"): "the Space tab",
}

# Edges the build no longer makes: (from, to) -> (cites, why).
EDGE_GONE = {
    ("paused", "countdown"): (["#44"], "Restart asks first: paused → restart-confirm → countdown; the count is still Restart's undo"),
}

EDGE_CHANGES = {
    ("landing", "live"): (["#29"], "the landing's demo block waits, hidden, for the real recording"),
    ("owner-page", "fully-live"): (["#46"], "no “Fully live ✓” on the page: the owner's upload line and its bar go when the video is up"),
}

# Each state's screen board on the shroom UI canvas: the probe's guess (machine-json/probe-canvas.mjs),
# settled in phase 4.
BOARD = {
    "all-set": "AllSet", "config": "Main", "countdown": "Countdown", "paused": "Paused", "discard-confirm": "DiscardConfirm",
    "recovered-take": "SavedTake", "mac-away": "AutoPause", "mic-lost": "AutoPause", "camera-lost": "AutoPause", "mic-permission-lost": "AutoPause",
    "quit-confirm": "QuitConfirm", "perm-mic": "WizardPerms", "perm-screen": "WizardPerms", "perm-priming": "WizardPerms", "idle-ready": "TrayFaces",
    "recording": "TrayFaces", "display-lost": "AutoPause", "disk-full-stop": "StoppedForYou", "recording-off": "DevicesOff", "landing": "Landing",
    "welcome": "Welcome", "idle-unprovisioned": "TrayFaces", "account": "WizardAccount", "token": "Wizard", "token-pick-account": "WizardPickAccount",
    "card": "WizardCard", "name-taken": "WizardName", "wiring": "WizardWiring", "wizard-offline": "WizardOffline", "done": "WizardPerms",
    "space-degraded": "SpaceRepair", "repair-mood": "SpaceRepair", "owner-page": "OwnerPage", "fully-live": "OwnerPageLive",
    "waiting-conn": "OwnerPagePending", "live": "RecipientLive", "on-its-way": "RecipientOnItsWay", "removed": "RecipientRemoved",
    "stopped-offline": "StoppedOffline", "library": "Library", "delete-confirm": "DeleteSheet", "settings-general": "SettingsGeneral",
    "settings-space": "SettingsSpace", "disconnect-confirm": "DisconnectSheet", "invite-born": "MembersInvited", "join-landing": "JoinLanding",
    "join-expired": "JoinLine", "join-used": "JoinLine", "member-welcome": "MemberWelcome", "join-switch-confirm": "SwitchConfirm",
    "joining": "Joining", "member-all-set": "AllSet",
}

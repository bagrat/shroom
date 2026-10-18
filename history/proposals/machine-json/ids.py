"""One id per state (machine-json phase 1): every name the old sources use, and the one it
becomes. Kebab-case, the spec's own notation for states. Imported by assemble.py.
"""

# The fixtures' ids that weren't kebab-case, and the code's (the core shows the fixtures'
# ids). Where the map had a kebab name for the same state, it wins.
RENAME = {
    "source view": "source-view",
    "token~repair-mood": "token-repair-mood",
    "name~v-clash": "name-v-clash",
    "space-degraded~no-token-mac": "space-degraded-no-token-mac",
    "discard-confirm (saved take)": "discard-confirm-saved-take",
    "permission-lost (microphone)": "mic-permission-lost",
    "red dot": "tray-red-dot",
    "pause bars": "tray-pause-bars",
    "amber dot": "tray-amber-dot",
    "Frost dot": "tray-frost-dot",
    # r5 drew the same face again under update-ready: one state now (see MERGE).
    "update-ready · Frost dot": "tray-frost-dot",
    "library · empty": "library-empty",
    "library · no match": "library-no-match",
    "library · offline": "library-offline",
    "library · the space can't be reached": "library-unreachable",
    "library · delete sheet": "delete-confirm",
    "library · delete sheet · another member's video": "delete-confirm-other-member",
    "library · delete sheet · several": "delete-confirm-several",
    "settings · General": "settings-general",
    "settings · Space": "settings-space",
    "settings · Space · inspection": "settings-space-inspection",
    "settings · Space · repair": "settings-space-repair",
    "settings · Space · owner-class-without-token": "settings-space-no-token",
    "settings · Space · member": "settings-space-member",
    "settings · Space · Disconnect sheet · owner, holds the token": "disconnect-confirm",
    "settings · Space · Disconnect sheet · owner-class, no token": "disconnect-confirm-no-token",
    "settings · Space · Leave sheet · member": "leave-confirm",
    "settings · Members": "settings-members",
    "update-ready · updating": "updating-app",
    "update failed": "update-failed",
    "space needs an update · member": "space-needs-update-member",
    "space needs an update · owner-class without the token": "space-needs-update-no-token",
    "Add another Mac": "add-another-mac",
    "member-remove-confirm · a member": "member-remove-confirm-member",
    "member-remove-confirm · a pending invite": "member-remove-confirm-invite",
    "member-remove-confirm · your other Mac": "member-remove-confirm-own-mac",
    "join-landing · owner view": "join-landing-owner-view",
    "join-landing · own-Mac variant": "join-landing-own-mac",
    "installing · member lane": "join-installing",
    "member-welcome · own-Mac variant": "member-welcome-own-mac",
    "joining · own-Mac variant": "joining-own-mac",
    "done · member": "member-done",
    "all-set · member": "member-all-set",
    "all-set · member, own-Mac variant": "member-all-set-own-mac",
    "join-expired · page": "join-expired-page",
    "join-used · page": "join-used-page",
    "join-switch-confirm · uploads draining": "join-switch-confirm-uploads",
    "join-switch-confirm · recovered take waiting": "join-switch-confirm-saved-take",
    "guided-agent overlay": "guided-agent-overlay",
}

# The map's names that weren't a fixture id. A list is a group (the map drew several states
# as one card); a port stands for the state on its far side.
MAP = {
    "flaunch": "first-launch",
    "idle": "idle-unprovisioned",
    "degraded": "space-degraded",
    "auto-pause": ["mac-away", "mic-lost", "camera-lost", "mic-permission-lost"],
    "settings-general": "settings-general",
    "settings-space": "settings-space",
    "exit-confirm": "disconnect-confirm",
    "delete-confirm": "delete-confirm",
    "updating-app": "updating-app",
    "update-failed": "update-failed",
    "add-another-mac": "add-another-mac",
    "join-installing": "join-installing",
    "member-done": "member-done",
    "member-all-set": "member-all-set",
    "source-view": "source-view",
    # ports, by the map's own captions (UX-MAP.html)
    "r4demo": "live",  # “Region 4 — the player page (the demo is live there)”
    "laterport": "idle-ready",  # “later life — any publish or periodic self-check”
    "portinvite": "invite-born",
    "portstop": "stopping",
    "portstopin": "stopping",
    "portcrash": "recording",  # “crash or forced shutdown mid-take — the next launch”
    "portlib": ["library", "settings-general"],
    "portlibin": "config",  # its edges go out of config's footer icons
    "portdelete": "delete-confirm",
    "porttomb": "removed",
    "portmembersin": "all-set",
    "portchapter": "perm-mic",
    "portconfig6": "config",
}

# Two fixture states that are one state: kept once, the second's facts folded in.
MERGE = {
    ("stopping", 3): ("stopping", 4),  # r3 and r4 both wrote the stop beat; Region 4 is its home
    ("update-ready · Frost dot", 5): ("Frost dot", 3),
}


def canon(name):
    """The one id for a fixture id or a code id."""
    return RENAME.get(name, name)

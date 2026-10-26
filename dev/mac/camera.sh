#!/bin/bash
# A camera for the hosted Mac, which has none: OBS's virtual camera, showing a test pattern.
#
#   camera.sh up      installs OBS, has macOS take its camera, and starts it on the pattern;
#                     fails unless macOS lists a camera at the end. Needs MAC_APP (the dev
#                     app, as a scenario has it): its camera check is how the camera is seen
#
# A scenario that needs a camera runs `bash dev/mac/camera.sh up` first (scenarios/camera.sh).
# The pattern is ffmpeg's testsrc2 — never anything of the room: the runner has none.
#
# Why it goes this way, from a held Mac (macOS 26.6.2, 2026-10-13):
#  - A camera is a camera extension now (the old plug-in folder is gone since macOS 14), and
#    the extension must be signed by an Apple developer; OBS's is (team 2MMRE5MTB8).
#  - macOS asks before it runs one: OBS's activation leaves it "activated waiting for user"
#    until a person switches it on in Login Items & Extensions and gives an admin's
#    password. Nobody knows the runner's own password, and it can't be reset (SecureToken).
#    So the switch is clicked and a throwaway admin, made here, answers the password. (With
#    System Integrity Protection off, as on this runner, writing the switch where macOS
#    keeps it, /Library/SystemExtensions/db.plist, reads as enabled but brings no camera:
#    runs 38451955522 and 38461018279.)
#  - OBS's camera offers one format, 1920 × 1080 at 60 fps, but sends frames at OBS's own
#    output size, which macOS doesn't scale up: OBS's output is set to that same size.
#  - Three things a person would click are taken out of the way first: Gatekeeper's
#    first-open question (the quarantine flag), OBS's own permissions sheet (its settings,
#    as OBS writes them once it has been shown) and the microphone question (a grant).
#
# RUNNER ONLY (lib.sh refuses anywhere else): it installs an app, writes grants and the
# system's extension switch, and may add a user — all to a Mac thrown away after the job.
set -u

HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/lib.sh"
OBS=/Applications/OBS.app
EXT=com.obsproject.obs-studio.mac-camera-extension
WORK="$RUNNER_TEMP/mac-camera"   # never uploaded: the throwaway admin's password is here
SHOTS="$OUT/camera"              # uploaded with the evidence
mkdir -p "$WORK" "$SHOTS" && chmod 700 "$WORK"

state() { systemextensionsctl list 2>/dev/null | grep -F "$EXT" | sed -n 's/.*\[\(.*\)\]$/\1/p' | head -n 1; }
cameras() { system_profiler SPCameraDataType 2>/dev/null | grep -c 'Model ID'; }
shot() { screencapture -x "$SHOTS/$1.png" >/dev/null 2>&1; }

install_obs() {
  [ -d "$OBS" ] || brew install --cask obs >>"$WORK/brew.log" 2>&1 || { note fail "camera: OBS installed" "$(tail -n 2 "$WORK/brew.log")"; return 1; }
  xattr -dr com.apple.quarantine "$OBS" 2>/dev/null
  note pass "camera: OBS installed" "$(plutil -extract CFBundleShortVersionString raw -o - "$OBS/Contents/Info.plist")"
}

# OBS's settings as it leaves them once its first run is behind it, a scene that plays the
# pattern, and the microphone granted so its default audio source asks nothing.
seed_obs() {
  local conf="$HOME/Library/Application Support/obs-studio"
  mkdir -p "$conf/basic/profiles/Untitled" "$conf/basic/scenes"
  ffmpeg -hide_banner -nostdin -y -loglevel error -f lavfi -i "testsrc2=size=1920x1080:rate=30" -t 20 \
    -c:v libx264 -preset veryfast -pix_fmt yuv420p "$WORK/pattern.mp4"
  printf '[General]\nMacOSPermissionsDialogLastShown=1\n' >"$conf/global.ini"
  # OBS's own remote control, on loopback, to start the virtual camera once macOS has
  # switched it on and OBS has been started again (start_virtual_camera).
  mkdir -p "$conf/plugin_config/obs-websocket"
  printf '{"first_load":false,"server_enabled":true,"server_port":4455,"auth_required":false,"alerts_enabled":false}\n' \
    >"$conf/plugin_config/obs-websocket/config.json"
  cat >"$conf/user.ini" <<'EOF'
[General]
Pre19Defaults=false
Pre21Defaults=false
Pre23Defaults=false
Pre24.1Defaults=false
FirstRun=true
MacOSPermissionsDialogLastShown=1
ConfirmOnExit=false

[Basic]
Profile=Untitled
ProfileDir=Untitled
SceneCollection=Untitled
SceneCollectionFile=Untitled.json
EOF
  cat >"$conf/basic/profiles/Untitled/basic.ini" <<'EOF'
[General]
Name=Untitled

[Video]
BaseCX=1920
BaseCY=1080
OutputCX=1920
OutputCY=1080
FPSType=0
FPSCommon=30
EOF
  cat >"$conf/basic/scenes/Untitled.json" <<EOF
{
  "name": "Untitled",
  "current_scene": "Scene",
  "current_program_scene": "Scene",
  "scene_order": [{ "name": "Scene" }],
  "sources": [
    { "id": "scene", "versioned_id": "scene", "name": "Scene",
      "settings": { "id_counter": 1, "items": [ { "name": "pattern", "id": 1, "visible": true,
        "pos": { "x": 0.0, "y": 0.0 }, "scale": { "x": 1.0, "y": 1.0 }, "align": 5 } ] } },
    { "id": "ffmpeg_source", "versioned_id": "ffmpeg_source", "name": "pattern",
      "settings": { "is_local_file": true, "local_file": "$WORK/pattern.mp4", "looping": true } }
  ]
}
EOF
  CLIENT=$(plutil -extract CFBundleIdentifier raw -o - "$OBS/Contents/Info.plist")
  CSREQ=$(requirement_hex "$(requirement_of "$OBS")")
  write_grant "$USER_DB" kTCCServiceMicrophone no
}

start_obs() {
  open -g -a "$OBS" --args --startvirtualcam --disable-shutdown-check --disable-updater --disable-missing-files-check
  sleep 15
  # The permissions sheet, should the settings not have kept it away: its Continue button.
  if windows | grep -q '^OBS: Review App Permissions'; then
    note info "camera: OBS's permissions sheet came up anyway" "Continue clicked"
    shot permissions-sheet
    cliclick c:501,651
    sleep 8
  fi
}

# Waits for a window of <owner> (a regex over the window list's owners), up to <seconds>.
wait_for() {
  local waited=0
  until windows | grep -Eq "^($1):"; do
    [ "$waited" -lt "$2" ] || return 1
    sleep 1
    waited=$((waited + 1))
  done
}

# The gray (0–255) of the screen at x,y: is what a click is aimed at drawn yet?
gray_at() {
  screencapture -x -R"$1,$2,1,1" "$WORK/pixel.png" >/dev/null 2>&1 || return 1
  ffmpeg -hide_banner -loglevel error -nostdin -i "$WORK/pixel.png" -f rawvideo -pix_fmt gray - 2>/dev/null | head -c 1 | od -An -tu1 | tr -d ' \n'
}

# The switch, as a person throws it: OBS's alert opens Login Items & Extensions, the OBS
# switch is clicked, and the throwaway admin answers the password. Places are those of the
# runner's 1024 × 768 screen, as the held Mac showed them. A loaded runner takes its time
# with every window (run 38461018279), so no click is made on a wait alone: each waits for
# what it's aimed at, and is made again until what it opens is up. The Camera Extensions
# sheet can sit empty for seconds after System Settings shows (run 40248053938: a click 4 s
# in hit the bare sheet, and no password was ever asked); its OBS row is drawn once the
# OBS icon's dark pixel is there (the bare sheet is 236 all over, the icon 136).
enable_by_hand() {
  local pw="$WORK/admin-password" waited=0 clicks=0
  [ -s "$pw" ] || { (umask 077; openssl rand -hex 16 >"$pw"); sudo sysadminctl -addUser camprobe -fullName camprobe -password "$(cat "$pw")" -admin >>"$WORK/enable.log" 2>&1; }
  wait_for UserNotificationCenter 60; sleep 1; shot before-settings
  until [ "$clicks" -ge 3 ]; do
    cliclick c:512,405; clicks=$((clicks + 1))
    wait_for "System Settings" 20 && break
  done
  wait_for "System Settings" 1 || { shot no-settings; echo "System Settings never came up ($clicks clicks)" >>"$WORK/enable.log"; return 1; }
  until [ "$(gray_at 322 272)" -lt 180 ] 2>/dev/null || [ "$waited" -ge 60 ]; do sleep 1; waited=$((waited + 1)); done
  echo "the OBS row drawn after $waited s" >>"$WORK/enable.log"
  shot settings
  clicks=0
  # A switch already blue (its track at 685,272 no longer the off gray, 205) is waiting on
  # its password: it isn't clicked again.
  until [ "$clicks" -ge 3 ]; do
    [ "$clicks" -gt 0 ] && [ "$(gray_at 685 272)" -lt 180 ] 2>/dev/null || { cliclick c:675,272; clicks=$((clicks + 1)); }
    wait_for SecurityAgent 30 && break
  done
  if ! wait_for SecurityAgent 1; then
    shot no-password
    echo "no password asked ($clicks clicks)" >>"$WORK/enable.log"
    return 1
  fi
  sleep 2; shot password
  cliclick c:512,343 kd:cmd t:a ku:cmd t:camprobe
  sleep 1
  cliclick c:512,375 "t:$(cat "$pw")"
  sleep 1
  cliclick c:570,417
  waited=0
  while [ "$(state)" != "activated enabled" ] && [ "$waited" -lt 60 ]; do sleep 1; waited=$((waited + 1)); done
  shot after-password
  cliclick c:694,569
  sleep 2
  pkill -x "System Settings" 2>/dev/null
}

# One request to OBS's remote control (obs-websocket 5): hello, identify, ask, answer.
obs_ask() {
  node -e '
    const [type] = process.argv.slice(1), ws = new WebSocket("ws://127.0.0.1:4455");
    const quit = (s, code) => { console.log(s); process.exit(code); };
    setTimeout(() => quit("no answer from OBS", 1), 10000);
    ws.onerror = () => quit("OBS is not listening", 1);
    ws.onmessage = (e) => {
      const m = JSON.parse(e.data);
      if (m.op === 0) ws.send(JSON.stringify({ op: 1, d: { rpcVersion: 1 } }));
      if (m.op === 2) ws.send(JSON.stringify({ op: 6, d: { requestType: type, requestId: "1" } }));
      if (m.op === 7) quit(JSON.stringify(m.d.responseData || m.d.requestStatus), m.d.requestStatus.result || m.d.requestStatus.code === 500 ? 0 : 1);
    };' "$1"
}

# The camera's brightness (0–255) as the dev app sees it: three seconds of its frames, by
# its own --camera-check, which already has the camera. OBS's own dark "not started"
# picture reads about 45; the pattern's bars about 128.
seen() {
  local out="$WORK/seen-$1.json"
  rm -f "$out"
  with_timeout 60 open -W -n "$MAC_APP" --args --camera-check "$out" >/dev/null 2>&1
  node -e 'try { const s = require(process.argv[1]).sample || {}; console.log(Math.round(s.mean_luma || 0)); } catch { console.log(0); }' "$out"
}

# OBS, ended and gone. Started again over one still quitting, `open` hands the launch to
# the one on its way out, and no OBS is left to answer (run 40245899605).
stop_obs() {
  local waited=0
  pkill -x OBS 2>/dev/null
  while pgrep -x OBS >/dev/null; do
    if [ "$waited" -ge 30 ]; then pkill -9 -x OBS 2>/dev/null; sleep 2; break; fi
    sleep 1; waited=$((waited + 1))
  done
}

# OBS started again — ended first, its Safe Mode marker cleared — and answering its remote
# control.
restart_obs() {
  local waited=0
  stop_obs
  rm -rf "$HOME/Library/Application Support/obs-studio/.sentinel"
  open -g -a "$OBS" --args --disable-shutdown-check --disable-updater --disable-missing-files-check
  until obs_ask GetVersion >/dev/null 2>&1; do
    [ "$waited" -lt 120 ] || return 1
    sleep 2; waited=$((waited + 2))
  done
}

# OBS finds the camera's system process as it starts the virtual camera: started before
# macOS has that process up, every start says "Could not find the virtual camera system
# process" (runs 38458869231, 38462001170). macOS brings it up in its own time after the
# switch, a long time on a loaded runner: OBS started again 4 s after the switch could
# still miss it for its three starts (runs 40240284787, 40240265681). So OBS waits for
# macOS to list the camera, is started again (macOS's crash dialog off), and a start that
# shows OBS's own picture three times gets a fresh OBS, up to three.
start_virtual_camera() {
  local round=0 tries waited=0 bright
  defaults write com.apple.CrashReporter DialogType none
  until [ "$(cameras)" -gt 0 ] || [ "$waited" -ge 120 ]; do sleep 2; waited=$((waited + 2)); done
  echo "macOS listed the camera after $waited s" >>"$WORK/obs.log"
  while [ "$round" -lt 3 ]; do
    if restart_obs; then
      tries=0
      while [ "$tries" -lt 3 ]; do
        obs_ask StartVirtualCam >>"$WORK/obs.log" 2>&1
        sleep 4
        bright=$(seen "$round-$tries")
        echo "OBS start $round, try $tries: brightness $bright" >>"$WORK/obs.log"
        [ "$bright" -gt 80 ] && return 0
        tries=$((tries + 1))
      done
    else
      echo "OBS start $round: no answer in 120 s" >>"$WORK/obs.log"
    fi
    round=$((round + 1))
  done
  return 1
}

# OBS, started again, can still say "The virtual camera is not installed" over a virtual
# camera that runs and shows the pattern; nothing closed that alert, and it sat over
# shroom's popover through the e2e run's recording (run 38585915506). It is OBS's modal
# panel (window layer 8): its OK, the one button in its bottom row, is clicked as a person
# would. The camera stays on. Each start that missed the camera left an alert of its own,
# stacked: they're clicked away one by one.
obs_alert_ok() {
  osascript -l JavaScript -e 'ObjC.import("CoreGraphics");
    const w = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)))
      .find((w) => /^OBS/.test(w.kCGWindowOwnerName) && w.kCGWindowLayer === 8);
    w ? Math.round(w.kCGWindowBounds.X + w.kCGWindowBounds.Width / 2) + "," + Math.round(w.kCGWindowBounds.Y + w.kCGWindowBounds.Height - 28) : ""' 2>/dev/null
}

dismiss_obs_alert() {
  local at tries=0
  at=$(obs_alert_ok)
  [ -n "$at" ] || { note info "camera: OBS's alert" "none up"; return 0; }
  shot obs-alert
  while [ -n "$at" ] && [ "$tries" -lt 8 ]; do
    cliclick "c:$at"
    sleep 2
    at=$(obs_alert_ok)
    tries=$((tries + 1))
  done
  if [ -z "$at" ]; then
    note info "camera: OBS's alert" "its OK clicked, the camera still on"
  else
    note info "camera: OBS's alert" "still up after $tries clicks — see camera/obs-alert.png"
  fi
}

up() {
  install_obs || return 1
  seed_obs
  start_obs
  local was
  was=$(state)
  note info "camera: OBS's camera after its start" "${was:-not asked for}"
  if [ "$was" != "activated enabled" ]; then
    enable_by_hand
    if [ "$(state)" = "activated enabled" ]; then
      note info "camera: switched on" "in System Settings, by the throwaway admin — $(grep -m 1 'drawn after' "$WORK/enable.log")"
    else
      note info "camera: not switched on" "$(tail -n 1 "$WORK/enable.log")"
    fi
  fi
  if start_virtual_camera; then
    note pass "camera: it shows the pattern" "$(tail -n 1 "$WORK/obs.log")"
    dismiss_obs_alert
  else
    note fail "camera: it shows the pattern" "OBS's own picture instead — $(tail -n 3 "$WORK/obs.log" | tr '\n' ' ')"
  fi
  shot obs
  systemextensionsctl list >"$SHOTS/extensions.txt" 2>&1
  system_profiler SPCameraDataType >"$SHOTS/cameras.txt" 2>&1
  if [ "$(cameras)" -gt 0 ]; then
    note pass "camera: macOS lists a camera" "$(grep -B1 'Model ID' "$SHOTS/cameras.txt" | head -n 1 | sed 's/^ *//; s/:$//') — extension $(state)"
  else
    note fail "camera: macOS lists a camera" "none — extension: $(state); see camera/*.png"
    return 1
  fi
}

case ${1:-} in
  up) up ;;
  *) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 64 ;;
esac

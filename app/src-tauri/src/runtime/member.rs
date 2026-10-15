//! The member lane (Region 6; WORKER-INTERFACE §4–§6): every `MemberCmd` the core issues
//! runs here through `shroom-cloud::member`, one call at a time, and answers the core
//! exactly once — its `MemberNews`, or `Failed` with its class (the core's deadline covers
//! a call that never returns). `ForgetInvite` and `Health` never answer (fire and forget).
//!
//! **Secrets (law 13).** A join link's code, a device secret and a minted credential come
//! from and go to the stash and `~/.shroom` (mode 600) only — never the core, a log line,
//! an IPC slice or anyone's argv. The core sees stash handles, hosts and kinds.
//!
//! **The disk (§5), act by act.** Join writes `join.json` before the call; a 200 writes
//! `device.key`, then `space.json` (`device_id` beside the identity), then deletes
//! `invite` and `join.json` — a write that fails keeps `join.json` for the replay. A switch
//! keeps the old space whole until the new files are written over it, then removes the old
//! space's other files and deregisters from the old host (best-effort, 3 s, the old key
//! held in memory). A launch with `join.json` replays the identical join. A dev build
//! switches only in a throwaway home, and keeps handed-in links on disk only there.
//!
//! **One space at a time (DECISIONS B41 h).** The lane is serial and every call for this
//! Mac's own space reads `space.json` when it runs; an answer that comes back after this
//! Mac left that space (a switch, Leave, Disconnect — each bumps `space_gen`) is dropped,
//! never reported.

use std::collections::HashMap;
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use shroom_cloud::device::{self, DeviceKey, InviteFile, JoinFile};
use shroom_cloud::member::{Answer, Class, IFACE_MIN, JoinNext, MeNext, NewInvite, Peek, Ping, Space, parse_link};
use shroom_core::effect::MemberCmd;
use shroom_core::event::{Event, JoinAnswer, MemberNews, PeekAnswer, StandingAnswer};
use shroom_core::time::Wall;
use shroom_core::world::{FailClass, InviteKind, LaunchKind, Op, PendingInvite, Role, Roster, RosterMac, RosterMember, SpaceIdentity};

use super::Runtime;
use super::stash::LinkKind;

/// How long a deregistration may hold a switch (§5: best-effort, 3 seconds).
const DEREGISTER: Duration = Duration::from_secs(3);

/// A join link's one stored form.
pub fn link_of(host: &str, code: &str) -> String {
    format!("https://{host}/join/{code}")
}

// ── the files (§5) ──

/// `device.key`, if this Mac has one.
pub fn device_key(rt: &Runtime) -> Option<DeviceKey> {
    rt.home.read_json::<DeviceKey>("device.key")
}

/// This Mac's device key for `host` — the one on disk, or a fresh one written first (a
/// token Mac's, before enroll: made once, reused on every retry, §5).
pub fn ensure_device_key(rt: &Runtime, host: &str) -> std::io::Result<DeviceKey> {
    if let Some(k) = device_key(rt).filter(|k| k.host == host) {
        return Ok(k);
    }
    let k = DeviceKey::generate(host);
    rt.home.write_json("device.key", &k)?;
    rt.log(&format!("member: a device key for {host} ({})", k.device));
    Ok(k)
}

/// `space.json`: the identity, plus this Mac's `device_id` when its key is for that space
/// (an additive field the core's reader ignores, §5).
pub fn write_space(rt: &Runtime, identity: &SpaceIdentity) -> std::io::Result<()> {
    let mut v = serde_json::to_value(identity).map_err(std::io::Error::other)?;
    if let (Some(k), Some(obj)) = (device_key(rt).filter(|k| k.host == identity.host), v.as_object_mut()) {
        obj.insert("device_id".into(), serde_json::Value::String(k.device));
    }
    rt.home.write_json("space.json", &v)
}

fn own_identity(rt: &Runtime) -> Option<SpaceIdentity> {
    rt.home.read_json::<SpaceIdentity>("space.json")
}

/// RFC 3339 (`2026-10-02T12:00:00Z`, `….000Z`) as unix milliseconds.
pub fn iso_ms(s: &str) -> Option<i64> {
    let s = s.trim().strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let d: Vec<i64> = date.split('-').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (hms, frac) = time.split_once('.').unwrap_or((time, "0"));
    let t: Vec<i64> = hms.split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let ([y, m, day], [h, min, sec]) = (d.as_slice(), t.as_slice()) else { return None };
    let ms: i64 = format!("{frac:0<3}")[..3].parse().ok()?;
    let yy = if *m <= 2 { y - 1 } else { *y };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let doy = (153 * (m + if *m > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + h * 3600 + min * 60 + sec) * 1000) + ms)
}

fn wall(s: &str) -> Wall {
    Wall(iso_ms(s).unwrap_or(0))
}

// ── arrivals: the installer's file, the deep link, a join.json at launch ──

/// Whether this build keeps a handed-in link on disk (`~/.shroom/invite`, §5). A dev build
/// without a throwaway `--home` keeps it in memory only: its home is the heartbeat's, and a
/// real invite opened on this Mac must never land a code there.
fn keeps_invites(rt: &Runtime) -> bool {
    !rt.dev || rt.throwaway
}

/// At launch, after `Booted`: a `join.json` is a join this Mac asked for (replayed
/// identically, B41 f/g); else `~/.shroom/invite` is an arrival when the installer opened
/// this launch (`--invite`), a remembered invite otherwise.
pub fn boot(rt: &Runtime, launch: LaunchKind) {
    if let Some(f) = rt.home.read_json::<JoinFile>("join.json") {
        let link = rt.stash.lock().unwrap().keep_link(LinkKind::Invite, link_of(&f.host, &f.code));
        rt.log(&format!("member: join.json found — the join replays (link {})", link.0));
        rt.post(Event::Member(MemberNews::JoinPending { link, host: f.host.clone() }));
        return;
    }
    check_invite_file(rt, matches!(launch, LaunchKind::Install { join: true }));
}

/// What the stash holds for an invite file that wasn't a join link: a stand-in, never the
/// file's text.
const UNREADABLE: &str = "shroom:unreadable-invite";

/// Reads `~/.shroom/invite` (either shape — the installer's `{v, link, arrived}` is
/// rewritten in the app's) and posts its arrival. `opened`: the installer's `--invite`
/// launch, or a reopen while running — the person just ran it; otherwise it's remembered.
/// An invite already held isn't posted again.
pub fn check_invite_file(rt: &Runtime, opened: bool) {
    let Ok(bytes) = std::fs::read(rt.home.path("invite")) else { return };
    let Some(f) = InviteFile::read(&bytes) else {
        // Text that isn't a join link (the installer's filter lets more through than the
        // app reads): the paste door's answer for it — the nameless dead-link face — and
        // the file goes, so it isn't met again (b4 #13). Its text is never logged or kept.
        if keeps_invites(rt) {
            rt.home.remove("invite");
        }
        let link = rt.stash.lock().unwrap().keep_link(LinkKind::Invite, UNREADABLE.into());
        rt.log("member: the invite file isn't a join link — the dead-link face, the file removed");
        rt.post(Event::Member(MemberNews::Unreadable { link, opened }));
        return;
    };
    let text = link_of(&f.host, &f.code);
    if rt.stash.lock().unwrap().holds_invite(&text) {
        return;
    }
    // The installer's shape becomes the app's (§5).
    if keeps_invites(rt) && serde_json::from_slice::<serde_json::Value>(&bytes).is_ok_and(|v| v.get("link").is_some()) {
        let _ = rt.home.write_json("invite", &f);
    }
    let link = rt.stash.lock().unwrap().keep_link(LinkKind::Invite, text);
    rt.log(&format!("member: an invite for {} {} (link {})", f.host, if opened { "arrived" } else { "is remembered" }, link.0));
    rt.post(Event::Member(MemberNews::Arrived { link, host: f.host, remembered: !opened }));
}

/// The page's Open shroom (`shroom://join/<host>/<code>`, RunEvent::Opened): written to
/// `~/.shroom/invite` and the stash, then posted as an arrival. Its text is never logged.
pub fn deep_link(rt: &Runtime, url: &str) {
    let Some((host, code)) = parse_link(url) else {
        rt.log("member: an opened link that isn't a join link — ignored");
        return;
    };
    let f = InviteFile { v: 1, code: code.clone(), host: host.clone(), inviter: None, own_mac: None, expires: None };
    if !keeps_invites(rt) {
        rt.log("member: a dev build keeps an opened link in memory only (no --home)");
    } else if let Err(e) = rt.home.write_json("invite", &f) {
        rt.log(&format!("member: couldn't keep the invite: {e}"));
    }
    let link = rt.stash.lock().unwrap().keep_link(LinkKind::Invite, link_of(&host, &code));
    rt.log(&format!("member: an invite for {host} arrived by the page's Open shroom (link {})", link.0));
    rt.post(Event::Member(MemberNews::Arrived { link, host, remembered: false }));
}

/// Writes (or fills) `~/.shroom/invite` for this link: a pasted one lands on disk before
/// anything else (DEVIATIONS #2), and peek fills in who and what (§5).
fn note_invite(rt: &Runtime, host: &str, code: &str, peek: Option<&Peek>) {
    if !keeps_invites(rt) {
        return;
    }
    let mut f = rt
        .home
        .read_json::<InviteFile>("invite")
        .filter(|f| f.host == host && f.code == code)
        .unwrap_or(InviteFile { v: 1, code: code.into(), host: host.into(), inviter: None, own_mac: None, expires: None });
    if let Some(p) = peek {
        f.inviter = Some(p.inviter.clone());
        f.own_mac = Some(p.kind == "own_mac");
        f.expires = Some(p.expires_at.clone());
    }
    let _ = rt.home.write_json("invite", &f);
}

// ── leaving a space ──

/// `DELETE devices/self` on `host` (§4.15) — best-effort: Leave, Disconnect and a switch
/// never wait on it more than `wait` (`None`: don't wait at all). Idempotent on the Worker.
pub fn deregister(rt: &Runtime, host: &str, key: DeviceKey, wait: Option<Duration>) {
    let Some(space) = rt.space(host) else {
        rt.log(&format!("member: this build doesn't reach {host}'s page — not deregistered"));
        return;
    };
    let (tx, rx) = channel();
    let rt2 = rt.clone();
    let host2 = host.to_string();
    std::thread::spawn(move || {
        let outcome = match space.leave(&key) {
            Answer::Ok(()) => "done".to_string(),
            Answer::Refused(r) => format!("refused {} {:?}", r.status, r.class),
            Answer::NotShroom { status } => format!("not shroom ({status})"),
            Answer::Unanswered(_) => "unanswered".into(),
        };
        rt2.log(&format!("member: deregistered from {host2}: {outcome}"));
        let _ = tx.send(());
    });
    if let Some(w) = wait {
        if rx.recv_timeout(w).is_err() {
            rt.log(&format!("member: {host} didn't answer the deregistration in time — going on"));
        }
    }
}

// ── the lane ──

/// A new link not yet answered 201: a retry after a lost reply resends the identical body
/// (§6.1 step 5). Its code never prints.
struct Pending {
    what: String,
    id: String,
    code: String,
}

struct Lane {
    rt: Runtime,
    pending: Option<Pending>,
}

pub fn spawn(rt: Runtime) -> Sender<(Instant, MemberCmd)> {
    let (tx, rx) = channel::<(Instant, MemberCmd)>();
    std::thread::Builder::new()
        .name("shroom-member".into())
        .spawn(move || {
            let mut lane = Lane { rt, pending: None };
            // A call can outlive the core's deadline for it (a slow link); the core then asks
            // again, queued behind the slow one. The slow one's answer satisfies the re-issue:
            // every issue gets one answer.
            let mut answered: HashMap<String, Instant> = HashMap::new();
            for (sent, cmd) in rx {
                let key = format!("{cmd:?}");
                if answered.get(&key).is_some_and(|at| *at > sent) {
                    lane.rt.log(&format!("member: {} already answered since it was asked — skipped", kind(&cmd)));
                    continue;
                }
                let answers = !matches!(cmd, MemberCmd::ForgetInvite | MemberCmd::Health { .. });
                if let Some(event) = lane.run(cmd, sent) {
                    lane.rt.post(event);
                }
                if answers {
                    answered.insert(key, Instant::now());
                }
            }
        })
        .expect("the member lane starts");
    tx
}

/// A command's kind for the log (never its label or a link).
fn kind(cmd: &MemberCmd) -> &'static str {
    match cmd {
        MemberCmd::Peek(_) => "peek",
        MemberCmd::Join(_) => "join",
        MemberCmd::ForgetInvite => "forget-invite",
        MemberCmd::Probe => "probe",
        MemberCmd::Roster => "roster",
        MemberCmd::Invite { .. } => "invite",
        MemberCmd::AddMac => "add-mac",
        MemberCmd::Renew(_) => "renew",
        MemberCmd::CancelInvite(_) => "cancel-invite",
        MemberCmd::RemoveMember(_) => "remove-member",
        MemberCmd::RemoveMac(_) => "remove-mac",
        MemberCmd::Mark(_) => "mark",
        MemberCmd::Health { .. } => "health",
    }
}

fn failed(op: Op, class: FailClass) -> Option<Event> {
    Some(Event::Failed { op, class })
}

/// A transport error that is really “no such host” while this Mac is online — the address
/// answers as no space at all (§6.7: NXDOMAIN while online reads as `space-gone`).
fn no_such_host(e: &str) -> bool {
    let e = e.to_ascii_lowercase();
    let dns = ["nodename nor servname", "failed to lookup", "no such host", "name or service not known", "dns"].iter().any(|m| e.contains(m));
    dns && super::net::online() == Some(true)
}

impl Lane {
    fn run(&mut self, cmd: MemberCmd, sent: Instant) -> Option<Event> {
        let rt = self.rt.clone();
        match cmd {
            MemberCmd::Peek(id) => self.peek(id),
            MemberCmd::Join(id) => self.join(id),
            MemberCmd::ForgetInvite => {
                // Only what was handed in before the core asked: a link that arrived since
                // (pasted, opened, the installer's file) is the next invite, not this one.
                let mut stash = rt.stash.lock().unwrap();
                let n = stash.forget_invites_before(sent);
                let file = rt.home.read_json::<InviteFile>("invite").map(|f| link_of(&f.host, &f.code));
                if file.is_some_and(|l| !stash.holds_invite(&l)) {
                    rt.home.remove("invite");
                }
                drop(stash);
                rt.log(&format!("member: the invite is behind us — {n} link(s) forgotten"));
                None
            }
            MemberCmd::Health { rec, refused } => {
                let Ok((identity, key, space)) = self.own() else { return None };
                let outcome = match space.health(&key, &rec.0, refused) {
                    Answer::Ok(()) => "reported",
                    Answer::Refused(_) => "refused",
                    Answer::NotShroom { .. } => "not shroom",
                    Answer::Unanswered(_) => "unanswered (the next report repeats it)",
                };
                rt.log(&format!("member: health {} for {} to {}: {outcome}", if refused { "refused" } else { "ok" }, super::short(&rec.0), identity.host));
                None
            }
            other => self.own_space_call(other),
        }
    }

    /// This Mac's own space: its identity, its device key (for that host) and a client.
    fn own(&self) -> Result<(SpaceIdentity, DeviceKey, Space), FailClass> {
        let rt = &self.rt;
        let Some(identity) = own_identity(rt) else { return Err(FailClass::Refused) };
        let Some(key) = device_key(rt).filter(|k| k.host == identity.host) else {
            rt.log("member: no device key for this Mac's space");
            return Err(FailClass::Refused);
        };
        let Some(space) = rt.space(&identity.host) else {
            rt.log(&format!("member: a dev build reaches {}'s page only through --space-base — refused", identity.host));
            return Err(FailClass::Refused);
        };
        Ok((identity, key, space))
    }

    /// Every call about this Mac's own space, dropped if this Mac left it meanwhile. A
    /// token Mac's device key is the space's only by enroll (best-effort at wiring, §6.11):
    /// one the space doesn't know (none yet, or a 401) enrolls — the token proves it — and
    /// the call runs once more; a token Mac is never “removed” by a 401.
    fn own_space_call(&mut self, cmd: MemberCmd) -> Option<Event> {
        let rt = self.rt.clone();
        let op = cmd_op(&cmd);
        let generation = *rt.space_gen.lock().unwrap();
        let token = token_mac(&rt);
        let mut enrolled = false;
        if token && own_identity(&rt).is_some_and(|i| device_key(&rt).is_none_or(|k| k.host != i.host)) {
            enrolled = true;
            enroll_here(&rt);
        }
        loop {
            let (identity, key, space) = match self.own() {
                Ok(t) => t,
                Err(class) => return failed(op, class),
            };
            UNENROLLED.set(false);
            let event = match cmd.clone() {
                MemberCmd::Probe => probe(&rt, &space, &key),
                MemberCmd::Roster => roster(&rt, &space, &key),
                MemberCmd::Invite { label } => self.invite(&identity, &space, &key, "member", Some(label), None),
                MemberCmd::AddMac => self.invite(&identity, &space, &key, "own_mac", None, None),
                MemberCmd::Renew(iid) => self.invite(&identity, &space, &key, "member", None, Some(iid)),
                MemberCmd::CancelInvite(iid) => removal(&rt, &space, &key, iid, |s, k, id| s.cancel_invite(k, id)),
                MemberCmd::RemoveMember(mid) => removal(&rt, &space, &key, mid, |s, k, id| s.remove_member(k, id)),
                MemberCmd::RemoveMac(did) => removal(&rt, &space, &key, did, |s, k, id| s.remove_device(k, id)),
                MemberCmd::Mark(on) => match space.mark(&key, on) {
                    Answer::Ok(now) => Some(Event::Member(MemberNews::Marked(now))),
                    other => classed(&rt, Op::Mark, "mark", &other),
                },
                MemberCmd::Peek(_) | MemberCmd::Join(_) | MemberCmd::ForgetInvite | MemberCmd::Health { .. } => None,
            };
            if token && UNENROLLED.take() && !enrolled {
                enrolled = true;
                if enroll_here(&rt) {
                    continue;
                }
            }
            // B41 h: an answer about a space this Mac has left never reaches the core.
            if *rt.space_gen.lock().unwrap() != generation || own_identity(&rt).is_none_or(|i| i.host != identity.host) {
                rt.log(&format!("member: an answer from {} after this Mac left it — dropped", identity.host));
                return None;
            }
            return event;
        }
    }

    /// `POST invites` — a member invite, an own-Mac link, or [ New link ] on a row (§6.1,
    /// §6.4). The code is made here, only its hash travels, and the link goes to the stash.
    fn invite(&mut self, identity: &SpaceIdentity, space: &Space, key: &DeviceKey, kind: &str, label: Option<String>, renew: Option<String>) -> Option<Event> {
        let rt = self.rt.clone();
        let what = format!("{kind}|{label:?}|{renew:?}");
        let (id, code) = match self.pending.as_ref().filter(|p| p.what == what) {
            Some(p) => (p.id.clone(), p.code.clone()),
            None => (renew.clone().unwrap_or_else(device::new_invite_id), device::new_join_code()),
        };
        self.pending = Some(Pending { what, id: id.clone(), code: code.clone() });
        let answer = match &renew {
            Some(iid) => space.renew(key, iid, &device::hash(&code)),
            None => space.invite(key, &NewInvite { id: id.clone(), code_hash: device::hash(&code), kind: kind.into(), label, inviter: identity.owner_name.clone() }),
        };
        match answer {
            Answer::Ok(row) => {
                self.pending = None;
                let link = rt.stash.lock().unwrap().keep_link(LinkKind::Born, link_of(&identity.host, &code));
                rt.log(&format!("member: a new {} link (row {}, link {})", if kind == "own_mac" { "own-Mac" } else { "member" }, row.id, link.0));
                let invite = PendingInvite {
                    id: row.id,
                    label: row.label,
                    created: wall(&row.created_at),
                    expires: wall(&row.expires_at),
                    own_mac: row.kind == "own_mac",
                };
                Some(Event::Member(MemberNews::Invited { invite, link }))
            }
            // A lost reply: the next try resends the identical body.
            Answer::Unanswered(_) => failed(Op::Invite, FailClass::Transport),
            other => {
                self.pending = None;
                classed(&rt, Op::Invite, "invite", &other)
            }
        }
    }

    // ── the join pipeline (§6.2) ──

    /// The held link's host and code — or, for text that isn't a join link at all, the
    /// host it names (it reads as a link that doesn't work).
    fn link(&self, id: shroom_core::world::SecretId) -> Result<(String, String), Option<String>> {
        let text = self.rt.stash.lock().unwrap().link(id).map(String::from);
        let Some(text) = text else { return Err(None) };
        parse_link(&text).ok_or_else(|| Some(loose_host(&text)))
    }

    fn peek(&mut self, id: shroom_core::world::SecretId) -> Option<Event> {
        let rt = self.rt.clone();
        let (host, code) = match self.link(id) {
            Ok(hc) => hc,
            Err(Some(host)) => {
                rt.log(&format!("member: link {} isn't a join link anyone made — it doesn't work", id.0));
                return Some(Event::Member(MemberNews::Peeked { link: id, peek: PeekAnswer::Expired { host, kind: InviteKind::Member, inviter: None } }));
            }
            Err(None) => {
                rt.log(&format!("member: link {} isn't held — nothing to peek", id.0));
                return failed(Op::Peek, FailClass::Unclassified);
            }
        };
        note_invite(&rt, &host, &code, None);
        let Some(space) = rt.space(&host) else {
            rt.log(&format!("member: a dev build reaches {host}'s page only through --space-base — refused"));
            return failed(Op::Peek, FailClass::Refused);
        };
        let missing = || Some(Event::Member(MemberNews::Peeked { link: id, peek: PeekAnswer::SpaceMissing { host: host.clone() } }));
        let iface = match space.ping() {
            Ping::Shroom { iface, .. } => iface,
            Ping::NotShroom { .. } => return missing(),
            Ping::Unanswered(e) if no_such_host(&e) => return missing(),
            Ping::Unanswered(_) => return failed(Op::Peek, FailClass::Transport),
        };
        let kind_of = |k: &str| if k == "own_mac" { InviteKind::OwnMac } else { InviteKind::Member };
        let peek = match space.peek(&code) {
            Answer::Ok(p) => {
                note_invite(&rt, &host, &code, Some(&p));
                let kind = kind_of(&p.kind);
                match p.state.as_str() {
                    "pending" => PeekAnswer::Pending { host: host.clone(), kind, inviter: p.inviter, current: iface >= IFACE_MIN },
                    "used" => PeekAnswer::Used { host: host.clone(), kind, inviter: p.inviter },
                    "expired" | "cancelled" => PeekAnswer::Expired { host: host.clone(), kind, inviter: Some(p.inviter) },
                    other => {
                        rt.log(&format!("member: peek answered a state this build doesn't read ({other})"));
                        return failed(Op::Peek, FailClass::Unclassified);
                    }
                }
            }
            Answer::Refused(r) if matches!(r.class, Class::Unknown | Class::Expired | Class::Cancelled) => PeekAnswer::Expired { host: host.clone(), kind: InviteKind::Member, inviter: None },
            Answer::Refused(r) => {
                rt.log(&format!("member: peek refused {} {:?}", r.status, r.class));
                return failed(Op::Peek, FailClass::Refused);
            }
            Answer::NotShroom { .. } => match space.ping() {
                Ping::NotShroom { .. } => PeekAnswer::SpaceMissing { host: host.clone() },
                _ => return failed(Op::Peek, FailClass::Refused),
            },
            Answer::Unanswered(_) => return failed(Op::Peek, FailClass::Transport),
        };
        rt.log(&format!("member: peek {} on {host}: {}", id.0, match &peek {
            PeekAnswer::Pending { kind, current, .. } => format!("pending ({kind:?}{})", if *current { "" } else { ", page behind" }),
            PeekAnswer::Expired { inviter, .. } => format!("expired{}", if inviter.is_none() { " (unknown)" } else { "" }),
            PeekAnswer::Used { .. } => "used".into(),
            PeekAnswer::SpaceMissing { .. } => "no space".into(),
        }));
        Some(Event::Member(MemberNews::Peeked { link: id, peek }))
    }

    fn join(&mut self, id: shroom_core::world::SecretId) -> Option<Event> {
        let rt = self.rt.clone();
        let joined = |answer: JoinAnswer| Some(Event::Member(MemberNews::Joined { link: id, answer }));
        let (host, code) = match self.link(id) {
            Ok(hc) => hc,
            Err(Some(_)) => return joined(JoinAnswer::Expired),
            Err(None) => return failed(Op::Join, FailClass::Unclassified),
        };
        let Some(space) = rt.space(&host) else {
            rt.log(&format!("member: a dev build reaches {host}'s page only through --space-base — refused"));
            return failed(Op::Join, FailClass::Refused);
        };
        // A switch wipes the old space's files: a dev build wipes only a throwaway home
        // (`wipe()`'s fence) — refused before anything is asked, so nothing is half done.
        if rt.dev && !rt.throwaway && switching(&rt, &host) {
            rt.log("member: a dev build switches spaces only in a --home throwaway — refused");
            return failed(Op::Join, FailClass::Refused);
        }
        // The fact file first (§5): the identical body replays after a crash.
        let mut file = match rt.home.read_json::<JoinFile>("join.json").filter(|f| f.host == host && f.code == code) {
            Some(f) => f,
            None => {
                let previous = device_key(&rt).filter(|k| k.host == host).map(|k| k.previous());
                let f = JoinFile { v: 1, host: host.clone(), code, device: device::new_device_id(), secret: device::new_device_secret(), name: super::os::sharing_name(), previous };
                if let Err(e) = rt.home.write_json("join.json", &f) {
                    rt.log(&format!("member: couldn't write join.json: {e}"));
                    return failed(Op::Join, FailClass::Unclassified);
                }
                f
            }
        };
        for attempt in 0..2 {
            let answer = space.join(&file);
            if let Answer::Unanswered(_) = answer {
                rt.log(&format!("member: join on {host} unanswered — it replays"));
                return failed(Op::Join, FailClass::Transport);
            }
            let next = JoinNext::of(answer);
            if next.clears_join_file() && !matches!(next, JoinNext::Joined(_)) {
                rt.home.remove("join.json");
            }
            return match next {
                JoinNext::Joined(j) => match self.land(&file, &j) {
                    Ok(identity) => joined(JoinAnswer::Joined(identity)),
                    // The space has this Mac, this Mac doesn't have the space yet: join.json
                    // stays, and the identical join replays (§4.5 answers it the same 200).
                    Err(e) => {
                        rt.log(&format!("member: joined {host}, but couldn't write {e} — join.json kept for the replay"));
                        failed(Op::Join, FailClass::Unclassified)
                    }
                },
                JoinNext::Expired => joined(JoinAnswer::Expired),
                JoinNext::Used => joined(JoinAnswer::Used),
                JoinNext::AlreadyHere => joined(JoinAnswer::AlreadyHere),
                JoinNext::Waiting => joined(JoinAnswer::Waiting),
                // The device id is someone else's: a fresh one, written first, once.
                JoinNext::NewDevice if attempt == 0 => {
                    file.device = device::new_device_id();
                    file.secret = device::new_device_secret();
                    if rt.home.write_json("join.json", &file).is_err() {
                        return failed(Op::Join, FailClass::Unclassified);
                    }
                    continue;
                }
                JoinNext::NewDevice => failed(Op::Join, FailClass::Unclassified),
                JoinNext::Reping => match space.ping() {
                    Ping::NotShroom { .. } => {
                        rt.home.remove("join.json");
                        joined(JoinAnswer::SpaceMissing)
                    }
                    Ping::Shroom { iface, .. } if iface < IFACE_MIN => {
                        rt.home.remove("join.json");
                        joined(JoinAnswer::Behind)
                    }
                    Ping::Shroom { .. } => failed(Op::Join, FailClass::Unclassified),
                    Ping::Unanswered(_) => failed(Op::Join, FailClass::Transport),
                },
                // 400, 403 `origin`, 409 `full`: a bug, logged — join.json stays (§4.5).
                JoinNext::Bug(r) => {
                    rt.log(&format!("member: join refused {} {:?} — kept for the replay", r.status, r.class));
                    failed(Op::Join, FailClass::Unclassified)
                }
            };
        }
        failed(Op::Join, FailClass::Unclassified)
    }

    /// join's 200 (§5). The new files land first — `device.key`, then `space.json`, each
    /// atomic — and only then does anything old go: a write that fails keeps `join.json`
    /// (the identical body replays at the next launch, and the Worker answers the same 200)
    /// and leaves the old space as it was. Then, on a switch, the old space's other files
    /// go and the old host hears this Mac leave (best-effort, 3 s, with the old key held in
    /// memory); last, the invite and `join.json`.
    fn land(&mut self, file: &JoinFile, j: &shroom_cloud::member::Joined) -> Result<SpaceIdentity, String> {
        let rt = self.rt.clone();
        let old = own_identity(&rt);
        let switch = switching(&rt, &file.host);
        let old_key = device_key(&rt);
        let identity = SpaceIdentity {
            account_id: j.space.account_id.clone().unwrap_or_default(),
            label: j.space.label.clone(),
            subdomain: j.space.subdomain.clone().unwrap_or_default(),
            host: file.host.clone(),
            bucket: j.space.bucket.clone().unwrap_or_default(),
            role: if j.role == "owner" { Role::OwnerClass } else { Role::Member },
            member_id: j.member.clone(),
            owner_name: Some(j.space.owner_name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| j.inviter.clone()),
            account_name: None,
            foreign_v: false,
        };
        // Held from the first new write to the last old removal: a lane writing for the old
        // space (the Library's cache) finishes before, or sees the new generation.
        let mut generation = rt.space_gen.lock().unwrap();
        rt.home.write_json("device.key", &file.device_key()).map_err(|e| format!("device.key: {e}"))?;
        if let Err(e) = write_space(&rt, &identity) {
            // Put the old key back (or none): the old space stays whole for the replay.
            match &old_key {
                Some(k) => {
                    let _ = rt.home.write_json("device.key", k);
                }
                None => rt.home.remove("device.key"),
            }
            return Err(format!("space.json: {e}"));
        }
        if switch {
            *generation += 1;
            for name in ["token.json", "library.json", "restore.json"] {
                rt.home.remove(name);
            }
            let _ = std::fs::remove_dir_all(rt.home.root.join("posters"));
        }
        drop(generation);
        if switch {
            for dir in super::takes::dirs(&rt.home.takes()) {
                let _ = std::fs::remove_file(dir.join("cred.json"));
            }
            rt.stash.lock().unwrap().forget_tokens();
            if let Some((o, k)) = old.as_ref().zip(old_key).filter(|(o, k)| k.host == o.host) {
                deregister(&rt, &o.host, k, Some(DEREGISTER));
            }
            rt.log(&format!("member: switched — the old space ({}) is gone from this Mac", old.as_ref().map(|o| o.host.as_str()).unwrap_or("half set up")));
        }
        rt.home.remove("invite");
        rt.home.remove("join.json");
        rt.log(&format!("member: joined {} as {:?} {} on device {}", identity.host, identity.role, identity.member_id, file.device));
        Ok(identity)
    }
}

/// Whether a join to `host` moves this Mac out of another space (N31): its space is
/// another, or it's half set up (a token, no space yet).
fn switching(rt: &Runtime, host: &str) -> bool {
    match own_identity(rt) {
        Some(o) => o.host != host,
        None => rt.home.path("token.json").exists(),
    }
}

/// The host a malformed link names, for its face.
pub(crate) fn loose_host(text: &str) -> String {
    let t = text.trim();
    let rest = t.strip_prefix("https://").or_else(|| t.strip_prefix("shroom://join/")).unwrap_or(t);
    rest.split('/').next().unwrap_or_default().to_ascii_lowercase()
}

fn cmd_op(cmd: &MemberCmd) -> Op {
    match cmd {
        MemberCmd::Peek(_) => Op::Peek,
        MemberCmd::Join(_) => Op::Join,
        MemberCmd::Probe | MemberCmd::ForgetInvite | MemberCmd::Health { .. } => Op::Standing,
        MemberCmd::Roster => Op::Roster,
        MemberCmd::Invite { .. } | MemberCmd::AddMac | MemberCmd::Renew(_) => Op::Invite,
        MemberCmd::CancelInvite(_) | MemberCmd::RemoveMember(_) | MemberCmd::RemoveMac(_) => Op::Remove,
        MemberCmd::Mark(_) => Op::Mark,
    }
}

thread_local! {
    /// The last call on this lane met a 401 on a token Mac: its device key isn't the
    /// space's yet (an enroll that didn't land) — never a removal.
    static UNENROLLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// This Mac holds its space's token (by its identity's role).
fn token_mac(rt: &Runtime) -> bool {
    own_identity(rt).is_some_and(|i| i.role == Role::TokenHolder)
}

/// A token Mac's enroll, here and now (the token proves the device key, §4.6).
fn enroll_here(rt: &Runtime) -> bool {
    match super::cloud::enroll_token_mac(rt) {
        Ok(e) => {
            rt.log(&format!("member: enrolled this token Mac ({} as {})", e.device, e.member));
            true
        }
        Err(e) => {
            rt.log(&format!("member: this token Mac's enroll didn't land ({e})"));
            false
        }
    }
}

/// A 401 on a token Mac: noted for the re-enroll, never read as a removal.
fn unenrolled(rt: &Runtime, r: &shroom_cloud::member::Refusal) -> bool {
    let yes = r.class == Class::Unauthorized && token_mac(rt);
    if yes {
        UNENROLLED.set(true);
    }
    yes
}

/// Any answer that isn't the one wanted, as the op's failure class (logged by kind only).
fn classed<T>(rt: &Runtime, op: Op, what: &str, a: &Answer<T>) -> Option<Event> {
    match a {
        Answer::Ok(_) => None,
        Answer::Refused(r) => {
            rt.log(&format!("member: {what} refused {} {:?}", r.status, r.class));
            if unenrolled(rt, r) {
                // Not this Mac's standing: the token's (the lane enrolls and asks again).
            } else if r.is_revoked() {
                // This identity is dead here: the face follows from `/me` (N34), said now.
                rt.post(Event::Member(MemberNews::Standing(StandingAnswer::Revoked)));
            }
            failed(op, FailClass::Refused)
        }
        Answer::NotShroom { status } => {
            rt.log(&format!("member: {what} answered as no shroom page ({status})"));
            failed(op, FailClass::Unclassified)
        }
        Answer::Unanswered(_) => failed(op, FailClass::Transport),
    }
}

/// The ping and `/me` (§4.2, §4.7): law 1's re-probe of this Mac's standing.
fn probe(rt: &Runtime, space: &Space, key: &DeviceKey) -> Option<Event> {
    let standing = |a: StandingAnswer| Some(Event::Member(MemberNews::Standing(a)));
    let iface = match space.ping() {
        Ping::Shroom { iface, .. } => iface,
        Ping::NotShroom { .. } => return standing(StandingAnswer::Gone),
        Ping::Unanswered(e) if no_such_host(&e) => return standing(StandingAnswer::Gone),
        Ping::Unanswered(_) => return failed(Op::Standing, FailClass::Transport),
    };
    if iface < IFACE_MIN {
        // The page has no `/me` yet: it needs an update (the needs-update line, §6.10).
        return standing(StandingAnswer::Fine { current: false, only_holder: None, mark: None });
    }
    let me = space.me(key);
    if let Answer::Refused(r) = &me {
        if unenrolled(rt, r) {
            rt.log("member: /me doesn't know this token Mac's device key yet");
            return failed(Op::Standing, FailClass::Refused);
        }
    }
    let answer = match MeNext::of(me) {
        MeNext::Me(m) => {
            let only_holder = m.owner.as_ref().map(|_| m.only_token_holder());
            StandingAnswer::Fine { current: true, only_holder, mark: m.owner.as_ref().map(|o| o.mark) }
        }
        MeNext::Dead => StandingAnswer::Revoked,
        MeNext::SpaceSide(_) => StandingAnswer::SpaceSide,
        MeNext::Reping => match space.ping() {
            Ping::NotShroom { .. } => StandingAnswer::Gone,
            _ => return failed(Op::Standing, FailClass::Unclassified),
        },
        MeNext::Keep => return failed(Op::Standing, FailClass::Transport),
    };
    rt.log(&format!("member: standing {}", match &answer {
        StandingAnswer::Fine { .. } => "fine",
        StandingAnswer::Revoked => "revoked",
        StandingAnswer::SpaceSide => "space-side",
        StandingAnswer::Gone => "gone",
    }));
    standing(answer)
}

/// `GET /members` (§4.11) as the core's roster.
fn roster(rt: &Runtime, space: &Space, key: &DeviceKey) -> Option<Event> {
    match space.members(key) {
        Answer::Ok(m) => {
            let r = Roster {
                macs: m.owner.devices.into_iter().map(|d| RosterMac { id: d.id, name: d.name.unwrap_or_default(), joined: wall(&d.joined_at), token: d.token, this_mac: d.this_mac }).collect(),
                members: m.members.into_iter().map(|x| RosterMember { id: x.id, label: x.label, joined: wall(&x.joined_at) }).collect(),
                invites: m
                    .invites
                    .into_iter()
                    .map(|i| PendingInvite { id: i.id, label: i.label, created: wall(&i.created_at), expires: wall(&i.expires_at), own_mac: i.kind == "own_mac" })
                    .collect(),
                labels: m.labels,
            };
            rt.log(&format!("member: roster — {} Mac(s), {} member(s), {} invite(s)", r.macs.len(), r.members.len(), r.invites.len()));
            Some(Event::Member(MemberNews::Roster(r)))
        }
        other => classed(rt, Op::Roster, "roster", &other),
    }
}

/// A removal or a cancel (§4.13–§4.15). A cancel that lost to a join (409 `used`) is the
/// row gone as an invite — the joined row comes with the roster read right after (§6.5).
fn removal(rt: &Runtime, space: &Space, key: &DeviceKey, id: String, call: impl Fn(&Space, &DeviceKey, &str) -> Answer<()>) -> Option<Event> {
    match call(space, key, &id) {
        Answer::Ok(()) => {
            rt.log(&format!("member: removed {id}"));
            Some(Event::Member(MemberNews::Removed(id)))
        }
        Answer::Refused(r) if r.class == Class::Used => {
            rt.log(&format!("member: {id} was used before the cancel landed — re-reading the roster"));
            rt.post(Event::Member(MemberNews::Removed(id)));
            roster(rt, space, key)
        }
        other => classed(rt, Op::Remove, "removal", &other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_workers_times() {
        assert_eq!(iso_ms("2026-10-02T12:00:00Z"), Some(1_790_942_400_000));
        assert_eq!(iso_ms("2026-10-02T12:00:00.250Z"), Some(1_790_942_400_250));
        assert_eq!(iso_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(iso_ms("2028-02-29T23:59:59Z"), Some(1_835_481_599_000));
        assert_eq!(iso_ms("yesterday"), None);
        assert_eq!(iso_ms("2026-10-02T12:00:00"), None, "only UTC");
    }

    /// The installer's hand-off, as install.sh writes it: a join link arrives; anything
    /// else is the dead-link face, and the file is gone (b4 #13).
    #[test]
    fn the_installers_file_arrives_or_is_the_dead_link_face() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("h");
        let (rt, rx) = Runtime::for_test(home.clone(), None, None);
        let write = |link: &str| std::fs::write(home.join("invite"), format!("{{\"v\":1,\"link\":\"{link}\",\"arrived\":\"2026-10-02T12:00:00Z\"}}\n")).unwrap();
        write("https://v.myshroom.workers.dev/join/AbCdEfGhIjKlMnOp");
        check_invite_file(&rt, true);
        let ev = rx.try_recv().unwrap().event;
        assert!(matches!(ev, Event::Member(MemberNews::Arrived { ref host, remembered: false, .. }) if host == "v.myshroom.workers.dev"), "{ev:?}");
        assert!(home.join("invite").exists(), "kept until it's behind us");

        write("abc");
        check_invite_file(&rt, true);
        let ev = rx.try_recv().unwrap().event;
        assert!(matches!(ev, Event::Member(MemberNews::Unreadable { opened: true, .. })), "{ev:?}");
        assert!(!home.join("invite").exists(), "the unreadable file is removed");
        let log = std::fs::read_to_string(home.join("logs/run.log")).unwrap();
        assert!(!log.contains("abc\""), "its text is never logged");
    }

    #[test]
    fn a_link_names_its_host_even_when_it_doesnt_work() {
        assert_eq!(link_of("v.x.workers.dev", "AbCdEfGhIjKlMnOp"), "https://v.x.workers.dev/join/AbCdEfGhIjKlMnOp");
        assert_eq!(loose_host("https://V.X.workers.dev/join/short"), "v.x.workers.dev");
        assert_eq!(loose_host("shroom://join/v.x.workers.dev/short"), "v.x.workers.dev");
    }
}

//! The Library window (N35): a list, newest first, merging this Mac's takes (recovered,
//! uploading, waiting, removed) with the space's records (the Library cache fact). State
//! words only when a video isn't live; a recovered take pins to the top. Search is local,
//! over titles. Offline, Copy link stays live; renaming and deleting grey out.

use crate::copy;
use crate::scene::{Action, Button, Choice, Gestures, LibraryBody, LibraryRowView, Sheet, Style, TextField, Tone, Window};
use crate::world::{Blocked, Confirm, CopyTarget, Landed, LibraryRow, LibraryScope, RecId, Role, TakeDir, WindowKind, World};

/// What a row is, by where it stands.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A recovered take: no link yet (law 3).
    Recovered(TakeDir),
    /// Uploading from this Mac.
    Uploading(u32),
    /// Uploading from another Mac.
    Elsewhere,
    WaitingConnection,
    /// The space can't take it right now: it can't go anywhere but Movies.
    WaitingSpace(TakeDir),
    /// Another Mac's delete stopped its upload; the take on this Mac is still its maker's.
    Removed(TakeDir),
    Live,
}

/// One merged row, before it's rendered.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub rec: RecId,
    pub title: Option<String>,
    pub created: crate::time::Wall,
    pub length_ms: Option<u64>,
    /// What plays once its saved edit is applied (N63), when that differs: shown in place
    /// of `length_ms`.
    pub edited_ms: Option<u64>,
    pub kind: Kind,
    /// Another member made it (owner-class Macs): their member id.
    pub other: Option<String>,
    /// Where its poster is (`LibraryRowView::poster`).
    pub poster: Option<String>,
}

impl Row {
    /// A published video: it has a link to copy, a page to open, and can be deleted.
    pub fn published(&self) -> bool {
        !matches!(self.kind, Kind::Recovered(_) | Kind::Removed(_))
    }

    fn key(&self) -> String {
        match &self.kind {
            Kind::Recovered(d) | Kind::Removed(d) => d.0.clone(),
            _ => self.rec.0.clone(),
        }
    }
}

/// This Mac's member id (owner-class Macs are member #1's).
fn me(w: &World) -> Option<&str> {
    w.space.identity.as_ref().map(|i| i.member_id.as_str())
}

/// Someone else made it: another member's video (a record without an owner is the space's
/// own, from before members).
fn others(w: &World, row: &LibraryRow) -> bool {
    row.owner.as_deref().is_some_and(|o| Some(o) != me(w))
}

/// *Mine · Everyone* shows: an owner-class Mac, once someone else has a video here (N33).
pub fn scoped(w: &World) -> bool {
    w.space.owner_class() && w.library.get().is_some_and(|rows| rows.iter().any(|r| others(w, r)))
}

/// Scope (N5, N33): members see their own; owner-class Macs see the whole space —
/// *Mine* first, once someone else records here.
fn in_scope(w: &World, row: &LibraryRow) -> bool {
    match w.space.role() {
        Some(Role::Member) => row.owner.as_deref() == me(w),
        // Mine is what the ledger says is mine: a row whose owner isn't known yet shows only
        // in Everyone, unlabeled (WORKER-INTERFACE §4.10, b4 #5).
        _ if scoped(w) && w.session.fields.library.scope == LibraryScope::Mine => row.owner.is_some() && !others(w, row),
        _ => true,
    }
}

/// Every row in this Mac's scope: recovered takes pinned on top (oldest first, as the
/// banner shows them), then everything else newest first.
pub fn rows(w: &World) -> Vec<Row> {
    let remote: Vec<&LibraryRow> = w.library.get().map(|v| v.iter().filter(|r| in_scope(w, r)).collect()).unwrap_or_default();
    let find = |rec: &RecId| remote.iter().find(|r| &r.rec == rec).copied();
    // Gone while its delete runs, and after it landed (a refresh begun before it doesn't
    // bring it back).
    let landed = &w.library_ops.landed;
    let gone = |rec: &RecId| w.library_ops.deleting.contains(rec) || matches!(landed.get(rec), Some((Landed::Deleted, _)));
    let title = |rec: &RecId, fallback: &str| {
        w.library_ops
            .renaming
            .get(rec)
            .cloned()
            .or_else(|| match landed.get(rec) {
                Some((Landed::Renamed(t), _)) => Some(t.clone()),
                _ => None,
            })
            .or_else(|| find(rec).map(|r| r.title.clone()))
            .unwrap_or_else(|| fallback.to_string())
    };
    // An edit saved from this Mac reads at once, over a refresh begun before it landed.
    let edited = |rec: &RecId| match w.library_ops.edited.get(rec) {
        Some((ms, _)) => *ms,
        None => find(rec).and_then(|r| r.edited_ms),
    };

    let pinned: Vec<Row> = w
        .saved
        .iter()
        .map(|s| Row {
            rec: s.rec.clone(),
            title: None,
            created: s.created,
            length_ms: Some(s.length_ms),
            edited_ms: None,
            kind: Kind::Recovered(s.dir.clone()),
            other: None,
            poster: Some(local_poster(&s.rec, &s.dir)),
        })
        .collect();

    let mut rest: Vec<Row> = Vec::new();
    for q in w.queue.iter().filter(|q| !gone(&q.rec)) {
        let r = find(&q.rec);
        let kind = match q.blocked {
            // The space can't take it — refused, degraded, or this Mac may no longer send
            // it (N34): it can only go to Movies. The exit sheets count the same takes (#8).
            _ if super::settings::stranded_take(w, q) => Kind::WaitingSpace(q.dir.clone()),
            Some(Blocked::Space) => Kind::WaitingSpace(q.dir.clone()),
            Some(Blocked::Offline) => Kind::WaitingConnection,
            None if w.online() == Some(false) => Kind::WaitingConnection,
            None => Kind::Uploading(if q.total == 0 { 0 } else { (q.done as u64 * 100 / q.total as u64) as u32 }),
        };
        rest.push(Row {
            rec: q.rec.clone(),
            title: Some(title(&q.rec, &q.title)),
            created: r.map(|r| r.created).unwrap_or(q.stopped),
            length_ms: r.and_then(|r| r.length_ms).or(q.length_ms),
            edited_ms: edited(&q.rec),
            kind,
            other: None,
            poster: Some(local_poster(&q.rec, &q.dir)),
        });
    }
    for q in &w.removed {
        rest.push(Row {
            rec: q.rec.clone(),
            title: Some(q.title.clone()),
            created: q.stopped,
            length_ms: q.length_ms,
            edited_ms: None,
            kind: Kind::Removed(q.dir.clone()),
            other: None,
            poster: Some(local_poster(&q.rec, &q.dir)),
        });
    }
    let local: Vec<&RecId> = w.queue.iter().chain(&w.removed).map(|q| &q.rec).chain(w.saved.iter().map(|s| &s.rec)).collect();
    for r in remote.iter().filter(|r| !local.contains(&&r.rec) && !gone(&r.rec)) {
        rest.push(Row {
            rec: r.rec.clone(),
            title: Some(title(&r.rec, &r.title)),
            created: r.created,
            length_ms: r.length_ms,
            edited_ms: edited(&r.rec),
            kind: if r.live { Kind::Live } else { Kind::Elsewhere },
            other: r.owner.clone().filter(|_| others(w, r)),
            poster: r.poster.then(|| r.rec.0.clone()),
        });
    }
    rest.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.rec.cmp(&b.rec)));
    pinned.into_iter().chain(rest).collect()
}

/// A take this Mac holds: its own `poster.jpg` first, then the space's (`<rec>/<dir>`).
fn local_poster(rec: &RecId, dir: &TakeDir) -> String {
    format!("{}/{}", rec.0, dir.0)
}

/// The rows the search lets through (local and instant, over titles only).
pub fn shown(w: &World) -> Vec<Row> {
    let q = w.session.fields.library.search.trim().to_lowercase();
    rows(w).into_iter().filter(|r| q.is_empty() || r.title.as_ref().is_some_and(|t| t.to_lowercase().contains(&q))).collect()
}

/// A row's title as shown (for rename's “emptied → the previous one comes back”).
pub fn title_of(w: &World, rec: &RecId) -> Option<String> {
    rows(w).into_iter().find(|r| &r.rec == rec).and_then(|r| r.title)
}

/// The published videos among these, in the Library's order — what Delete… may take.
pub fn deletable(w: &World, recs: &[RecId]) -> Vec<RecId> {
    rows(w).into_iter().filter(|r| r.published() && recs.contains(&r.rec)).map(|r| r.rec).collect()
}

pub fn derive(w: &World) -> Option<Window<LibraryBody>> {
    let door = w.session.windows.get(&WindowKind::Library)?;
    if !w.window_visible(WindowKind::Library) {
        return None;
    }
    let f = &w.session.fields.library;
    let all = rows(w);
    let shown = shown(w);
    let offline = w.online() == Some(false);
    let search = f.search.trim();
    let mut faces = Vec::new();
    if offline {
        faces.push("library-offline");
    }
    // A member Mac its space can't take videos from (N34): copy and Movies stay live.
    let unreachable = w.space.role() == Some(Role::Member) && super::members::stranded_by_space(w);
    if unreachable {
        faces.push("library-unreachable");
    }
    let scope = if scoped(w) {
        [(LibraryScope::Mine, copy::mine()), (LibraryScope::Everyone, copy::everyone())]
            .into_iter()
            .map(|(s, label)| Choice { action: Action::SetScope(s), label, selected: f.scope == s, enabled: true, detail: None })
            .collect()
    } else {
        vec![]
    };
    // “No videos yet” is a fact about the space, so it waits for the bucket to say so: an
    // empty cache on a Mac that has never listed this space (a second Mac, after a rejoin)
    // shows an empty list, not the line, until a refresh answers (law 1).
    let empty = if all.is_empty() {
        w.library_ops.refreshed.is_some().then(|| {
            faces.push("library-empty");
            copy::library_empty()
        })
    } else if shown.is_empty() {
        faces.push("library-no-match");
        Some(copy::no_match(search))
    } else {
        None
    };
    let sheet = sheet(w, &all);
    let renaming = f.renaming.is_some();
    let (ret, esc) = match (&sheet, renaming) {
        (Some(_), _) => (None, Some(Action::Back)),
        // A focused field's Return belongs to it (law 8): Enter saves the title — online;
        // offline, renaming waits until you're back (the field greys, Return saves nothing).
        (None, true) => ((!offline && !unreachable).then_some(Action::CommitRename), Some(Action::CancelRename)),
        (None, false) => (None, None),
    };
    Some(Window {
        id: "library",
        faces,
        body: LibraryBody {
            title: copy::library(),
            search: TextField { placeholder: copy::search_titles(), value: f.search.clone(), enabled: true, edits: f.search_edits },
            scope,
            notice: offline.then(copy::library_offline),
            empty,
            rows: shown.iter().map(|r| view(w, r)).collect(),
            footer: (!all.is_empty()).then(|| copy::video_count(all.len() as u64)),
            sheet,
        },
        ret,
        esc,
        activates: door.asked,
    })
}

fn view(w: &World, r: &Row) -> LibraryRowView {
    let f = &w.session.fields.library;
    let online = w.online() != Some(false);
    // A member Mac its space can't take videos from: only Copy link and Save to Movies stay
    // live (N34) — what would ask the space waits for it.
    let reachable = online && !(w.space.role() == Some(Role::Member) && super::members::stranded_by_space(w));
    let cut_off = !reachable && online;
    let mid_take = w.take_open();
    // An edited video's length is what plays (N63).
    let len = r.edited_ms.or(r.length_ms).map(copy::length);
    let meta = copy::row_meta(&copy::day(r.created, w.mac.utc_offset_min), len.as_deref());
    let rec = r.rec.clone();
    // What would put something new on screen waits for the take (law 4).
    let after_take = |b: Button| if mid_take { Button { enabled: false, note: Some(copy::after_take()), ..b } } else { b };
    // An update applying quits the app any moment now: Get link waits (law 4).
    let updating = |b: Button| if w.update.applying && b.enabled { Button { enabled: false, note: Some(copy::updating()), ..b } } else { b };
    let copied = matches!(&w.session.copied, Some((CopyTarget::Link(c), _)) if *c == rec);
    let copy_btn = || Button::new(Action::CopyRow(rec.clone()), if copied { copy::copied() } else { copy::copy_link() }, Style::Plain);
    let open_btn = || after_take(Button::new(Action::OpenPage(rec.clone()), copy::open_page(), Style::Link).enabled(!cut_off));
    let delete_btn = || Button::new(Action::DeleteRow(rec.clone()), copy::delete_ellipsis(), Style::RedText).enabled(reachable);
    let rename = f.renaming.as_ref().filter(|(x, _)| *x == rec).map(|(_, v)| TextField { placeholder: r.title.clone().unwrap_or_default(), value: v.clone(), enabled: reachable, edits: f.title_edits });
    let no_gestures = Gestures { click: None, cmd_click: None, shift_click: None, title_click: None, double_click: None, copy: None };

    let (state, tone, progress) = match &r.kind {
        Kind::Recovered(_) => (Some(copy::row_interrupted(len.as_deref().unwrap_or("0:00"))), Some(Tone::Amber), None),
        Kind::Uploading(p) => (Some(copy::row_uploading(*p)), None, Some(*p)),
        Kind::Elsewhere => (Some(copy::row_uploading_elsewhere()), None, None),
        Kind::WaitingConnection => (Some(copy::row_waiting_connection()), None, None),
        Kind::WaitingSpace(_) => (Some(copy::row_waiting_space()), Some(Tone::Amber), None),
        Kind::Removed(_) => (Some(copy::row_removed()), None, None),
        Kind::Live => (None, None, None),
    };

    let (buttons, hover, menu, gestures) = match &r.kind {
        // Both mirror N25's banner: their doors always visible, never on hover only (only
        // a published row's Open page and Delete… wait for hover or selection).
        Kind::Recovered(dir) => (
            // Its one filled button leads (law 8's single-action pattern, N25's banner). On a
            // member Mac its space can't take videos from, Get link would ask that space:
            // it greys, and Save to Movies (N34's rescue) is offered beside it (b4 #9).
            {
                let get = updating(after_take(Button::new(Action::GetLinkRow(dir.clone()), copy::get_link(), Style::Filled).enabled(!cut_off)));
                let mut b = vec![get];
                if cut_off {
                    b.push(super::members::save_button(w, Action::SaveRowToMovies(dir.clone()), &[dir], copy::save_to_movies(), Style::Plain));
                }
                b.push(Button::new(Action::DiscardRow(dir.clone()), copy::discard_ellipsis(), Style::RedText));
                b
            },
            vec![],
            vec![],
            no_gestures,
        ),
        Kind::Removed(dir) => (
            vec![
                super::members::save_button(w, Action::SaveRowToMovies(dir.clone()), &[dir], copy::save_to_movies(), Style::Plain),
                Button::new(Action::DiscardRow(dir.clone()), copy::discard_ellipsis(), Style::RedText),
            ],
            vec![],
            vec![],
            no_gestures,
        ),
        _ => {
            let mut buttons = vec![copy_btn()];
            if let Kind::WaitingSpace(dir) = &r.kind {
                buttons.push(super::members::save_button(w, Action::SaveRowToMovies(dir.clone()), &[dir], copy::save_to_movies(), Style::Plain));
            }
            let mut menu = vec![copy_btn()];
            // Copy thumbnail sits under Copy link, on a live row (N46).
            if matches!(r.kind, Kind::Live) {
                menu.push(Button::new(Action::CopyThumbRow(rec.clone()), copy::copy_thumbnail(), Style::Plain));
            }
            menu.extend([
                open_btn(),
                Button::new(Action::StartRename(rec.clone()), copy::rename(), Style::Plain).enabled(reachable),
                delete_btn(),
            ]);
            let gestures = Gestures {
                click: Some(Action::SelectRow(rec.clone())),
                cmd_click: Some(Action::ToggleRow(rec.clone())),
                shift_click: Some(Action::ExtendRow(rec.clone())),
                title_click: reachable.then(|| Action::StartRename(rec.clone())),
                double_click: (!mid_take && !cut_off).then(|| Action::OpenPage(rec.clone())),
                copy: Some(Action::CopyRow(rec.clone())),
            };
            (buttons, vec![open_btn(), delete_btn()], menu, gestures)
        }
    };
    LibraryRowView {
        key: r.key(),
        title: r.title.clone(),
        rename,
        meta,
        state,
        tone,
        progress,
        pinned: matches!(r.kind, Kind::Recovered(_)),
        selected: r.published() && f.selected.contains(&r.rec),
        gestures,
        buttons,
        hover,
        menu,
        // *Everyone*'s rows add the member's label (N33).
        member: r.other.as_deref().and_then(|o| super::members::label_of(w, o)),
        poster: r.poster.clone(),
    }
}

fn sheet(w: &World, all: &[Row]) -> Option<Sheet> {
    match w.session.confirm.as_ref()? {
        // N23's law: no Return default, Esc ≡ Cancel, Delete red-outlined, rightmost.
        Confirm::Delete(recs) => {
            let titled: Vec<&Row> = all.iter().filter(|r| recs.contains(&r.rec)).collect();
            if titled.is_empty() {
                return None;
            }
            let (id, mut faces, title, line) = if titled.len() == 1 {
                let t = titled[0].title.clone().unwrap_or_default();
                ("delete-confirm", vec![], copy::delete_one(&t), copy::delete_one_line())
            } else {
                ("delete-confirm", vec!["delete-confirm-several"], copy::delete_many(titled.len() as u64), copy::delete_many_line())
            };
            let mut lines = vec![line];
            // The owner deletes anything (N5) — and is told whose it is.
            if let [row] = titled.as_slice() {
                if let Some(label) = row.other.as_deref().and_then(|o| super::members::label_of(w, o)) {
                    faces.push("delete-confirm-other-member");
                    lines.push(copy::their_video(&label));
                }
            }
            let online = w.online() != Some(false);
            Some(Sheet {
                id,
                faces,
                title,
                lines,
                blocked: None,
                buttons: vec![
                    Button::new(Action::Back, copy::cancel(), Style::Plain),
                    Button::new(Action::ConfirmDelete, copy::delete(), Style::RedOutline).enabled(online),
                ],
            })
        }
        // A take that never reached anyone: N25's question, not the delete sheet — asked
        // in the Library, where the click was (DEVIATIONS #15).
        Confirm::DiscardRow(dir) => {
            let row = all.iter().find(|r| matches!(&r.kind, Kind::Recovered(d) | Kind::Removed(d) if d == dir))?;
            let line = match (&row.kind, row.length_ms) {
                (_, Some(len)) => copy::toss_saved(&copy::length(len)),
                _ => copy::toss_unknown_length(),
            };
            Some(Sheet {
                id: "discard-confirm",
                faces: vec!["discard-confirm-saved-take"],
                title: copy::toss_title(),
                lines: vec![line],
                blocked: None,
                buttons: vec![
                    Button::new(Action::Back, copy::back(), Style::Plain),
                    Button::new(Action::ConfirmDiscardRow, copy::discard(), Style::RedOutline),
                ],
            })
        }
        _ => None,
    }
}

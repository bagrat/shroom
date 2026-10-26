//! Suggested to clean up (N68): the space's play counts, read to its end, become a line in
//! the Library and a view of its own — each suggestion with why it's suggested, Keep, and a
//! one-tap Delete that waits out its Undo line — and Settings' Videos tab (N79, was General's
//! group Your videos): the space's play-count switch and this Mac's suggestions switch; what
//! the space costs is Space used, in the Space tab.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::sync::Arc;

use shroom_core::effect::{Effect, LibCmd, PersistOp};
use shroom_core::event::{Event, LibraryNews, OsNews, Surface};
use shroom_core::scene::{Action, CleanupBody, Scene, SettingsPane, SpaceBody, Style, VideosGroup};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use support::*;

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_790_000_000_000;

fn rec(id: &str) -> RecId {
    RecId(id.into())
}

fn remote(id: &str, title: &str, days_ago: i64, owner: &str) -> LibraryRow {
    LibraryRow { rec: rec(id), title: title.into(), created: Wall(NOW - days_ago * DAY), length_ms: Some(600_000), live: true, owner: Some(owner.into()), poster: false, edited_ms: None, pinned: false }
}

fn space_rows() -> Vec<LibraryRow> {
    vec![
        remote("billing", "Code walkthrough: billing service", 380, "m1"),
        remote("acme", "Customer call recap (Acme)", 45, "m1"),
        remote("roadmap", "Q3 roadmap walkthrough", 160, "m1"),
        remote("standup", "Standup", 2, "m1"),
    ]
}

fn suggestion(id: &str, bytes: u64, plays: u32, quiet_days: u32) -> Suggestion {
    Suggestion { rec: rec(id), bytes, plays, quiet_days, stopped_early: false }
}

fn three() -> Vec<Suggestion> {
    vec![suggestion("billing", 3_200_000_000, 4, 300), suggestion("acme", 1_800_000_000, 0, 45), suggestion("roadmap", 1_300_000_000, 41, 120)]
}

fn answer(rows: Vec<Suggestion>, space_bytes: u64) -> Event {
    Event::Library(LibraryNews::Suggested(Suggestions { counting: true, space_bytes, rows, ..Default::default() }))
}

/// The Library open over the space, the bucket's refresh answered, and the space's
/// suggestions known.
fn library(rows: Vec<Suggestion>, space_bytes: u64) -> World {
    let mut w = ready();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w.library = Fact::known(Arc::new(space_rows()), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    w.suggestions = Fact::known(Suggestions { counting: true, space_bytes, rows, ..Default::default() }, Mono(960));
    w
}

fn with_popover(mut w: World) -> World {
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

fn on_tab(w: World, tab: SettingsTab) -> World {
    let mut w = w;
    w.prefs.settings_tab = Some(tab);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

fn videos_tab(w: World) -> World {
    on_tab(w, SettingsTab::Videos)
}

fn space_tab(w: World) -> World {
    on_tab(w, SettingsTab::Space)
}

fn member(mut w: World) -> World {
    let id = w.space.identity.as_mut().unwrap();
    id.role = Role::Member;
    id.member_id = "m_Mher____________".into();
    w.space.token = false;
    if let Fact::Known { v, .. } = &mut w.library {
        for r in Arc::make_mut(v).iter_mut() {
            r.owner = Some("m_Mher____________".into());
        }
    }
    w
}

trait Drive {
    fn lib(&mut self, action: Action) -> &mut Self;
    fn set(&mut self, action: Action) -> &mut Self;
}

impl Drive for Run {
    fn lib(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Library, action, rev: 0 })
    }
    fn set(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Settings, action, rev: 0 })
    }
}

fn view(s: &Scene) -> &CleanupBody {
    s.library.as_ref().and_then(|l| l.body.cleanup.as_ref()).expect("Suggested to clean up is open")
}

fn titles(s: &Scene) -> Vec<String> {
    view(s).rows.iter().map(|r| r.title.clone().unwrap_or_default()).collect()
}

fn line(s: &Scene) -> Option<String> {
    s.library.as_ref().and_then(|l| l.body.cleanup_line.as_ref()).map(|l| l.line.clone())
}

fn videos(s: &Scene) -> &VideosGroup {
    match &s.settings.as_ref().expect("Settings open").body.pane {
        SettingsPane::Videos(g) => g,
        other => panic!("not Videos: {other:?}"),
    }
}

fn space(s: &Scene) -> &SpaceBody {
    match &s.settings.as_ref().expect("Settings open").body.pane {
        SettingsPane::Space(b) => b,
        other => panic!("not Space: {other:?}"),
    }
}

fn button<'a>(s: &'a Scene, title: &str, action: &Action) -> &'a shroom_core::scene::Button {
    let row = view(s).rows.iter().find(|r| r.title.as_deref() == Some(title)).unwrap_or_else(|| panic!("no row {title}"));
    row.buttons.iter().find(|b| &b.action == action).unwrap_or_else(|| panic!("no {action:?} on {title}"))
}

#[test]
fn the_library_says_so_in_one_line_and_review_opens_the_view() {
    let mut r = Run::new(library(three(), 34_000_000_000));
    assert_eq!(line(&r.scene).as_deref(), Some("Three videos might be worth deleting — they take 6.3 GB."));
    let l = r.scene.library.as_ref().unwrap().body.cleanup_line.clone().unwrap();
    assert_eq!((l.review.label.as_str(), l.dismiss.label.as_str()), ("Review…", "Not now"));
    assert!(r.scene.library.as_ref().unwrap().body.cleanup.is_none(), "the list first");

    r.lib(Action::ReviewCleanup);
    assert_eq!(r.arm.edge, Some(("library", "library-cleanup")));
    assert!(r.ids().contains(&"library-cleanup"));
    assert!(line(&r.scene).is_none(), "the line is the view's door, not on it");
    let v = view(&r.scene);
    assert_eq!(v.title, "Suggested to clean up");
    assert_eq!(v.back.label, "‹ Library");
    assert_eq!(titles(&r.scene), ["Code walkthrough: billing service", "Customer call recap (Acme)", "Q3 roadmap walkthrough"], "the space's order: costliest per remaining play first");
    assert_eq!(v.rows[0].meta, format!("{} · 3.2 GB · 5¢ a month · Watched 4 times, the last 10 months ago", shroom_core::copy::day(Wall(NOW - 380 * DAY), 0)));
    assert!(v.rows[1].meta.ends_with("1.8 GB · 3¢ a month · Nobody has watched it"));
    assert!(v.rows[2].meta.ends_with("1.3 GB · 2¢ a month · Watched 41 times, the last 4 months ago"));
    assert_eq!(v.intro.as_deref(), Some("These might be worth deleting: rarely watched lately, and taking real room. Deleting all three frees 6.3 GB — about 9¢ a month off your Cloudflare bill."));
    assert_eq!(v.foot.as_deref(), Some("A pinned video is never suggested; unpin it in the Library. Plays are counted in your own Cloudflare account."));
    let all = v.delete_all.clone().unwrap();
    assert_eq!((all.label.as_str(), all.style), ("Delete all three…", Style::RedText), "law 9: red text, never a button");
    let labels: Vec<_> = v.rows[0].buttons.iter().map(|b| (b.label.as_str(), b.style)).collect();
    assert_eq!(labels, [("Open page", Style::Link), ("Pin", Style::Plain), ("Delete", Style::RedText)]);
    let w = r.scene.library.as_ref().unwrap();
    assert_eq!((w.ret.clone(), w.esc.clone()), (None, None), "no Return default; ‹ Library is the way back");

    r.lib(Action::Back);
    assert_eq!(r.arm.edge, Some(("library-cleanup", "library")));
    assert!(r.scene.library.as_ref().unwrap().body.cleanup.is_none());
    assert!(line(&r.scene).is_some(), "the line again, the suggestions unchanged");
}

#[test]
fn a_one_tap_delete_waits_out_its_undo_line() {
    let mut w = library(three(), 34_000_000_000);
    w.session.fields.library.cleanup = true;
    let mut r = Run::new(w);
    r.lib(Action::DeleteSuggested(rec("acme")));
    assert!(r.effects.is_empty(), "nothing is sent while Undo stands");
    assert_eq!(titles(&r.scene), ["Code walkthrough: billing service", "Q3 roadmap walkthrough"], "the row leaves at once");
    let undo = view(&r.scene).undo.clone().unwrap();
    assert_eq!((undo.line.as_str(), undo.undo.label.as_str()), ("Deleted “Customer call recap (Acme)”.", "Undo"));
    assert!(r.scene.library.as_ref().unwrap().body.rows.iter().all(|x| x.title.as_deref() != Some("Customer call recap (Acme)")), "gone from the list behind it too");

    r.lib(Action::UndoDelete);
    assert!(r.effects.is_empty() && view(&r.scene).undo.is_none());
    assert_eq!(titles(&r.scene).len(), 3, "back where it was");

    r.lib(Action::DeleteSuggested(rec("acme")));
    let due = r.scene.next_wake.expect("the Undo line's end wakes the core");
    r.tick_to(due.0);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("acme")]))), "sent once its Undo line goes");
    assert!(view(&r.scene).undo.is_none());
    assert!(r.w.library_ops.deleting.contains(&rec("acme")));
    let acme = r.w.suggestions.get().unwrap().rows.iter().find(|s| s.rec == rec("acme")).map(|s| s.bytes).unwrap();
    r.send(Event::Library(LibraryNews::Deleted(vec![rec("acme")])));
    assert_eq!(titles(&r.scene).len(), 2);
    let after = r.w.suggestions.get().unwrap();
    assert_eq!(after.space_bytes, 34_000_000_000 - acme, "the space's size drops as the delete lands, before the next read (Space used, the money line)");
    assert!(after.rows.iter().all(|s| s.rec != rec("acme")));
}

#[test]
fn a_second_tap_sends_the_first_and_closing_the_window_sends_what_waits() {
    let mut w = library(three(), 34_000_000_000);
    w.session.fields.library.cleanup = true;
    let mut r = Run::new(w);
    r.lib(Action::DeleteSuggested(rec("acme")));
    r.lib(Action::DeleteSuggested(rec("billing")));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("acme")]))), "the first can't be undone once another takes the line");
    assert_eq!(view(&r.scene).undo.as_ref().unwrap().line, "Deleted “Code walkthrough: billing service”.");
    r.send(Event::WindowClosed(WindowKind::Library));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("billing")]))), "the tap meant delete: it goes as the window closes");
}

#[test]
fn pin_takes_a_video_off_the_list_for_good_and_a_failed_pin_brings_it_back() {
    let mut w = library(three(), 34_000_000_000);
    w.session.fields.library.cleanup = true;
    let mut r = Run::new(w);
    r.lib(Action::PinRow(rec("roadmap"), true));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Pin(vec![rec("roadmap")], true))));
    assert_eq!(titles(&r.scene).len(), 2, "it leaves while the write runs");
    r.send(Event::Failed { op: Op::Pin, class: FailClass::Transport });
    assert_eq!(titles(&r.scene).len(), 3, "a pin that didn't land: the row is back");
    r.lib(Action::PinRow(rec("roadmap"), true));
    r.send(Event::Library(LibraryNews::Pinned(vec![rec("roadmap")], true)));
    assert_eq!(titles(&r.scene).len(), 2);
    // A refresh begun before the keep landed still lists it: the landed keep reads over it.
    r.send(answer(three(), 34_000_000_000));
    assert_eq!(titles(&r.scene).len(), 2);
}

#[test]
fn delete_all_asks_once_with_its_own_sheet_then_holds_them_all_on_one_undo_line() {
    let mut w = library(three(), 34_000_000_000);
    w.session.fields.library.cleanup = true;
    let mut r = Run::new(w);
    r.lib(Action::DeleteAllSuggested);
    assert_eq!(r.arm.edge, Some(("library-cleanup", "delete-confirm-cleanup")));
    assert!(r.ids().contains(&"delete-confirm-cleanup"));
    let sheet = r.scene.library.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.id, "delete-confirm-cleanup");
    assert_eq!(sheet.title, "Delete all three suggested videos?");
    assert_eq!(sheet.lines, ["That frees 6.3 GB. They're removed everywhere. Their links stay, and say the videos are gone."]);
    let labels: Vec<_> = sheet.buttons.iter().map(|b| (b.label.as_str(), b.style)).collect();
    assert_eq!(labels, [("Cancel", Style::Plain), ("Delete all three", Style::RedOutline)], "N23's law: the destructive one rightmost, red-outlined");
    assert_eq!(r.scene.library.as_ref().unwrap().ret, None, "no Return default (law 9)");

    r.lib(Action::Back);
    assert_eq!(r.arm.edge, Some(("delete-confirm-cleanup", "library-cleanup")));
    assert_eq!(titles(&r.scene).len(), 3, "Cancel: nothing moved");

    r.lib(Action::DeleteAllSuggested);
    r.lib(Action::ConfirmDeleteSuggested);
    assert_eq!(r.arm.edge, Some(("delete-confirm-cleanup", "library-cleanup")));
    assert!(r.effects.is_empty(), "nothing is sent while Undo stands");
    let v = view(&r.scene);
    assert_eq!(v.empty.as_deref(), Some("Nothing to clean up right now."), "every row leaves at once");
    assert!(v.intro.is_none() && v.delete_all.is_none());
    let undo = v.undo.clone().unwrap();
    assert_eq!((undo.line.as_str(), undo.undo.label.as_str()), ("Deleted three videos.", "Undo"));

    r.lib(Action::UndoDelete);
    assert_eq!(titles(&r.scene).len(), 3, "Undo brings every row back");
    assert!(r.effects.is_empty());

    r.lib(Action::DeleteAllSuggested);
    r.lib(Action::ConfirmDeleteSuggested);
    let due = r.scene.next_wake.expect("the Undo line's end wakes the core");
    r.tick_to(due.0);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("billing"), rec("acme"), rec("roadmap")]))), "sent together once the line goes");
    assert!(view(&r.scene).undo.is_none());
}

#[test]
fn delete_all_of_two_reads_both_and_leaves_out_one_pinned_since() {
    let mut w = library(three(), 34_000_000_000);
    w.session.fields.library.cleanup = true;
    let mut r = Run::new(w);
    r.lib(Action::PinRow(rec("roadmap"), true));
    assert_eq!(view(&r.scene).delete_all.as_ref().unwrap().label, "Delete both…");
    r.lib(Action::DeleteAllSuggested);
    let sheet = r.scene.library.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.title, "Delete both suggested videos?");
    assert_eq!(sheet.buttons[1].label, "Delete both");
    r.lib(Action::ConfirmDeleteSuggested);
    assert_eq!(view(&r.scene).undo.as_ref().unwrap().line, "Deleted both videos.");
    r.send(Event::WindowClosed(WindowKind::Library));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Delete(vec![rec("billing"), rec("acme")]))), "the pinned one stays; closing sends what waits");
}

#[test]
fn the_line_put_away_stays_away_until_the_suggestions_change() {
    let mut r = Run::new(library(three(), 34_000_000_000));
    r.lib(Action::DismissCleanup);
    assert!(line(&r.scene).is_none());
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.cleanup_dismissed == ["acme", "billing", "roadmap"])), "remembered on this Mac");
    r.send(answer(three(), 34_000_000_000));
    assert!(line(&r.scene).is_none(), "the same suggestions: still away");
    let mut more = three();
    more.push(suggestion("standup", 900_000_000, 0, 31));
    r.send(answer(more, 35_000_000_000));
    assert_eq!(line(&r.scene).as_deref(), Some("Four videos might be worth deleting — they take 7.2 GB."), "a new one: the line is back");
}

#[test]
fn the_money_line_is_honest_about_the_free_ten_gigabytes() {
    let open = |rows, total| {
        let mut w = library(rows, total);
        w.session.fields.library.cleanup = true;
        Run::new(w)
    };
    let r = open(three(), 34_000_000_000);
    assert!(view(&r.scene).intro.as_deref().unwrap().ends_with("frees 6.3 GB — about 9¢ a month off your Cloudflare bill."));
    let costs = |r: &Run| view(&r.scene).rows.iter().map(|row| row.meta.split(" · ").nth(2).unwrap_or_default().to_string()).collect::<Vec<_>>();
    assert_eq!(costs(&r), ["5¢ a month", "3¢ a month", "2¢ a month"], "past the free 10 GB, each row its own cost: its size at the list price");
    let r = open(three(), 9_000_000_000);
    assert!(view(&r.scene).intro.as_deref().unwrap().ends_with("frees 6.3 GB. Your space is inside Cloudflare's free 10 GB, so this saves nothing yet."));
    assert!(view(&r.scene).rows.iter().all(|row| !row.meta.contains("a month")), "inside the free 10 GB, no row has a cost");
    let r = open(three(), 12_000_000_000);
    assert!(view(&r.scene).intro.as_deref().unwrap().ends_with("frees 6.3 GB and brings your space back inside Cloudflare's free 10 GB."));
    assert_eq!(costs(&r), ["5¢ a month", "3¢ a month", "2¢ a month"], "partly past: still the size at the list price");
    let r = open(vec![suggestion("billing", 3_200_000_000, 4, 300)], 34_000_000_000);
    assert_eq!(view(&r.scene).intro.as_deref(), Some("These might be worth deleting: rarely watched lately, and taking real room. Deleting it frees 3.2 GB — about 5¢ a month off your Cloudflare bill."));
    assert!(view(&r.scene).delete_all.is_none(), "one suggestion: its own Delete is enough");
    let r = Run::new({
        let mut w = member(library(three(), 6_300_000_000));
        w.session.fields.library.cleanup = true;
        w
    });
    assert_eq!(view(&r.scene).intro.as_deref(), Some("These might be worth deleting: rarely watched lately, and taking real room. Deleting all three frees 6.3 GB."), "the bill is the owner's");
    let r = Run::new({
        let mut w = member(library(three(), 34_000_000_000));
        w.session.fields.library.cleanup = true;
        w
    });
    assert!(view(&r.scene).rows.iter().all(|row| !row.meta.contains("a month")), "a member's rows say no money either");
}

#[test]
fn offline_pin_and_delete_grey_and_open_page_stays() {
    let mut w = library(three(), 34_000_000_000);
    w.session.fields.library.cleanup = true;
    let mut r = Run::new(w);
    r.send(Event::Os(OsNews::NetPath(false)));
    assert!(!button(&r.scene, "Q3 roadmap walkthrough", &Action::PinRow(rec("roadmap"), true)).enabled);
    assert!(!button(&r.scene, "Q3 roadmap walkthrough", &Action::DeleteSuggested(rec("roadmap"))).enabled);
    assert!(!view(&r.scene).delete_all.as_ref().unwrap().enabled);
    r.lib(Action::DeleteSuggested(rec("roadmap")));
    assert!(view(&r.scene).undo.is_none(), "a greyed Delete does nothing");
}

#[test]
fn the_suggestions_are_read_on_entry_and_by_scope() {
    // An owner-class Mac reads the whole space's; the line waits for the answer.
    let mut w = library(vec![], 0);
    w.suggestions = Fact::Unknown;
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    let mut r = Run::new(with_popover(w));
    r.act(Action::OpenLibrary);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Suggest { everyone: true })));
    assert!(line(&r.scene).is_none());
    r.send(answer(three(), 34_000_000_000));
    assert!(line(&r.scene).is_some());
    // A member's Mac reads its own.
    let mut w = member(library(vec![], 0));
    w.suggestions = Fact::Unknown;
    w.session.windows.remove(&WindowKind::Library);
    let mut r = Run::new(with_popover(w));
    r.act(Action::OpenLibrary);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Suggest { everyone: false })));
    // Suggestions off on this Mac: the Library reads none, and says nothing.
    let mut w = library(three(), 34_000_000_000);
    w.prefs.suggest_cleanup = false;
    w.session.windows.remove(&WindowKind::Library);
    w.suggestions = Fact::Unknown;
    let mut r = Run::new(with_popover(w));
    r.act(Action::OpenLibrary);
    assert!(!r.has(|e| matches!(e, Effect::Library(LibCmd::Suggest { .. }))));
    r.send(answer(three(), 34_000_000_000));
    assert!(line(&r.scene).is_none());
    // …unless the space's auto-cleanup is on: its warning reaches every Mac (N75).
    let mut w = library(three(), 34_000_000_000);
    w.prefs.suggest_cleanup = false;
    if let Fact::Known { v, .. } = &mut w.suggestions {
        v.autoclean.days = 30;
    }
    w.suggestions = Fact::known(w.suggestions.get().unwrap().clone(), Mono(10));
    w.session.windows.remove(&WindowKind::Library);
    let mut r = Run::new(with_popover(w));
    r.act(Action::OpenLibrary);
    assert!(r.has(|e| matches!(e, Effect::Library(LibCmd::Suggest { .. }))));
}

#[test]
fn settings_counts_plays_by_the_spaces_switch_and_suggests_by_this_macs() {
    let mut r = Run::new(videos_tab(library(three(), 34_000_000_000)));
    let s = r.scene.settings.as_ref().unwrap();
    assert!(s.faces.contains(&"settings-videos"), "{:?}", s.faces);
    let tabs: Vec<_> = s.body.tabs.iter().map(|t| (t.label.as_str(), t.selected)).collect();
    assert_eq!(tabs, [("General", false), ("Videos", true), ("Space", false), ("Members", false)], "the group's heading is the tab's name now (N79)");
    let g = videos(&r.scene);
    assert_eq!((g.count.label.as_str(), g.count.on, g.count.enabled), ("Count plays", true, true));
    assert_eq!(g.count_hint, "How often each video is played and how far, kept in your Cloudflare account. No cookies, nothing about who watched.");
    assert_eq!((g.suggest.label.as_str(), g.suggest.on, g.suggest.enabled), ("Suggest videos to clean up", true, true));
    assert_eq!(g.suggest_hint, "In the Library, videos that might be worth deleting: rarely watched lately, and taking real room.");

    r.set(Action::SetPlays(false));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Plays(false))));
    let g = videos(&r.scene);
    assert!(!g.count.on && !g.count.enabled, "it reads as asked, greyed until the space answers");
    r.send(Event::Library(LibraryNews::Plays(false)));
    let g = videos(&r.scene);
    assert!(!g.count.on && g.count.enabled);
    r.set(Action::SetPlays(true));
    r.send(Event::Failed { op: Op::Plays, class: FailClass::Transport });
    assert!(!videos(&r.scene).count.on, "a switch that didn't land reads as the space has it");

    r.set(Action::SetSuggestCleanup(false));
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if !p.suggest_cleanup)));
    assert!(!videos(&r.scene).suggest.on);
    assert!(line(&r.scene).is_none(), "the Library's line goes with it");
}

#[test]
fn space_used_is_in_the_space_tab_on_an_owner_class_mac() {
    // N79 (Bagrat, 2026-10-25): what the space holds and costs sits with the space.
    let r = Run::new(space_tab(library(three(), 34_000_000_000)));
    let b = space(&r.scene);
    assert_eq!((b.used_label.as_deref(), b.used.as_deref()), (Some("Space used"), Some("34 GB — about 36¢ a month past Cloudflare's free 10 GB")));
    let r = Run::new(space_tab(library(vec![], 8_200_000_000)));
    assert_eq!(space(&r.scene).used.as_deref(), Some("8.2 GB — inside Cloudflare's free 10 GB"));
    let r = Run::new(space_tab(library(vec![], 134_500_000_000)));
    assert_eq!(space(&r.scene).used.as_deref(), Some("135 GB — about $1.87 a month past Cloudflare's free 10 GB"));
    // General carries none of it any more.
    let r = Run::new(on_tab(library(three(), 34_000_000_000), SettingsTab::General));
    assert!(matches!(r.scene.settings.as_ref().unwrap().body.pane, SettingsPane::General(_)));
    // Entering Space reads the space's size when nothing has answered since (law 1).
    let mut w = space_tab(library(vec![], 0));
    w.suggestions = Fact::Unknown;
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    let mut r = Run::new(w);
    r.set(Action::SettingsTab(SettingsTab::Space));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Suggest { everyone: true })));
    // …and entering Videos does too: its switches read the space.
    let mut w = on_tab(library(vec![], 0), SettingsTab::General);
    w.suggestions = Fact::Unknown;
    let mut r = Run::new(w);
    r.set(Action::SettingsTab(SettingsTab::Videos));
    assert_eq!(r.arm.edge, Some(("settings-general", "settings-videos")));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Suggest { everyone: true })));
}

#[test]
fn a_members_mac_shows_the_spaces_switch_greyed_and_no_bill() {
    let r = Run::new(videos_tab(member(library(three(), 6_300_000_000))));
    let g = videos(&r.scene);
    assert!(g.count.on && !g.count.enabled, "the space's switch is owner-class's");
    assert!(g.suggest.enabled);
    let tabs: Vec<_> = r.scene.settings.as_ref().unwrap().body.tabs.iter().map(|t| t.label.clone()).collect();
    assert_eq!(tabs, ["General", "Videos", "Space"], "Videos on every Mac; Members owner-class only");
    let r = Run::new(space_tab(member(library(three(), 6_300_000_000))));
    assert_eq!(space(&r.scene).used, None, "the bill is the owner's");
    // Before the space has said, the switch waits greyed, and Space used waits.
    let mut w = videos_tab(library(vec![], 0));
    w.suggestions = Fact::Unknown;
    let g = Run::new(w).scene.clone();
    assert!(!videos(&g).count.enabled);
    let mut w = space_tab(library(vec![], 0));
    w.suggestions = Fact::Unknown;
    assert!(space(&Run::new(w).scene).used.is_none());
}

#[test]
fn every_n68_op_has_a_deadline_and_a_late_answer_after_disconnect_is_stale() {
    for op in [Op::Suggest, Op::Pin, Op::Plays, Op::Autoclean] {
        assert_eq!(op.deadline_ms(), 60_000, "{op:?}");
    }
    let mut w = library(three(), 34_000_000_000);
    w.space.identity = None;
    let mut r = Run::new(w);
    r.send(answer(vec![], 0));
    assert!(r.w.suggestions.get().is_some_and(|s| !s.rows.is_empty()), "an answer about a space this Mac left changes nothing");
}

#[test]
fn reasons_read_as_row_facts() {
    use shroom_core::copy::cleanup_reason as why;
    assert_eq!(why(0, 400, false), "Nobody has watched it");
    assert_eq!(why(1, 150, false), "Watched once, 5 months ago");
    assert_eq!(why(2, 45, true), "Watched twice, stopped early, the last 45 days ago");
    assert_eq!(why(12, 800, false), "Watched 12 times, the last 2 years ago");
    assert_eq!(shroom_core::copy::size(780_000_000), "780 MB");
    assert_eq!(shroom_core::copy::size(999_700_000), "1 GB");
    assert_eq!(shroom_core::copy::size(10_400_000_000), "10 GB");
    assert_eq!(shroom_core::copy::delete_all(2), "Delete both…");
    assert!(shroom_core::copy::cleanup_frees(2, 4_500_000_000).ends_with("Deleting both frees 4.5 GB."));
}

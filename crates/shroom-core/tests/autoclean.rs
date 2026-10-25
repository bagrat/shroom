//! Auto-cleanup (N75): the space's setting in Settings · General, set on the Mac that holds
//! the setup key and read everywhere else; three days ahead, the Library's amber line, each
//! going row's Pin, Review… with what's going first, and the popover's banner with Pin both;
//! once a round has gone, a line that says so, with OK. The space's page does the deleting.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::sync::Arc;

use shroom_core::effect::{CloudCmd, Effect, LibCmd, PersistOp};
use shroom_core::event::{CloudNews, Event, LibraryNews, Surface};
use shroom_core::scene::{Action, Scene, SettingsPane, Style, VideosGroup};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use support::*;

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_790_000_000_000;
/// Three days on: the date the going videos go.
const DUE: i64 = NOW + 3 * DAY;

fn rec(id: &str) -> RecId {
    RecId(id.into())
}

fn remote(id: &str, title: &str, days_ago: i64, owner: &str) -> LibraryRow {
    LibraryRow { rec: rec(id), title: title.into(), created: Wall(NOW - days_ago * DAY), length_ms: Some(435_000), live: true, owner: Some(owner.into()), poster: false, edited_ms: None, pinned: false }
}

fn space_rows() -> Vec<LibraryRow> {
    vec![
        remote("r502", "Where the 502s come from", 10, "m1"),
        remote("weekly", "Weekly update, Sep 18", 33, "m1"),
        remote("design", "Design review: settings sheet", 48, "m1"),
        remote("billing", "Code walkthrough: billing service", 380, "m1"),
    ]
}

fn going(id: &str, bytes: u64, plays: u32, quiet_days: u32) -> Going {
    Going { rec: rec(id), bytes, plays, quiet_days, stopped_early: false, deletes_at: Wall(DUE) }
}

fn facts(days: u32, going: Vec<Going>, last: Option<Cleaned>) -> Suggestions {
    Suggestions {
        counting: true,
        space_bytes: 34_000_000_000,
        rows: vec![Suggestion { rec: rec("billing"), bytes: 3_200_000_000, plays: 4, quiet_days: 300, stopped_early: false }],
        going,
        idle: vec![Idle { rec: rec("weekly"), quiet_days: 31 }, Idle { rec: rec("design"), quiet_days: 33 }, Idle { rec: rec("billing"), quiet_days: 300 }],
        autoclean: AutoClean { days, last },
    }
}

fn two_going() -> Vec<Going> {
    vec![going("weekly", 640_000_000, 2, 31), going("design", 1_100_000_000, 0, 33)]
}

fn space(s: Suggestions) -> World {
    let mut w = ready();
    w.library = Fact::known(Arc::new(space_rows()), Mono(950));
    w.library_ops.refreshed = Some(Mono(950));
    w.suggestions = Fact::known(s, Mono(960));
    w
}

fn library(s: Suggestions) -> World {
    let mut w = space(s);
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w
}

fn popover(s: Suggestions) -> World {
    let mut w = space(s);
    w.session.popover = Some(Door::asked(Mono(900)));
    w
}

fn videos_tab(s: Suggestions) -> World {
    let mut w = space(s);
    w.prefs.settings_tab = Some(SettingsTab::Videos);
    w.session.windows.insert(WindowKind::Settings, Door::asked(Mono(900)));
    w
}

trait Drive {
    fn lib(&mut self, action: Action) -> &mut Self;
    fn set(&mut self, action: Action) -> &mut Self;
    fn pop(&mut self, action: Action) -> &mut Self;
}

impl Drive for Run {
    fn lib(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Library, action, rev: 0 })
    }
    fn set(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Settings, action, rev: 0 })
    }
    fn pop(&mut self, action: Action) -> &mut Self {
        self.send(Event::Act { surface: Surface::Popover, action, rev: 0 })
    }
}

fn day(at: i64) -> String {
    shroom_core::copy::day(Wall(at), 0)
}

fn going_line(s: &Scene) -> Option<String> {
    s.library.as_ref().and_then(|l| l.body.going_line.as_ref()).map(|l| l.line.clone())
}

fn row_going(s: &Scene, title: &str) -> Option<String> {
    s.library.as_ref().unwrap().body.rows.iter().find(|r| r.title.as_deref() == Some(title)).unwrap().going.clone()
}

fn banner(s: &Scene) -> Option<(&'static str, String, Vec<String>)> {
    s.popover.as_ref().and_then(|p| p.banner.as_ref()).map(|b| (b.id, b.line.clone(), b.buttons.iter().map(|x| x.label.clone()).collect()))
}

fn videos(s: &Scene) -> &VideosGroup {
    match &s.settings.as_ref().expect("Settings open").body.pane {
        SettingsPane::Videos(g) => g,
        other => panic!("not Videos: {other:?}"),
    }
}

#[test]
fn three_days_ahead_the_library_says_which_videos_go_and_each_has_its_pin() {
    let mut r = Run::new(library(facts(30, two_going(), None)));
    assert_eq!(going_line(&r.scene).unwrap(), format!("Two videos will be deleted on {} — nobody has watched them in 30 days.", day(DUE)));
    let l = r.scene.library.as_ref().unwrap().body.going_line.clone().unwrap();
    let labels: Vec<_> = l.buttons.iter().map(|b| b.label.as_str()).collect();
    assert_eq!(labels, ["Review…"], "no Not now: it stays until they're pinned, watched or gone");
    assert!(r.scene.library.as_ref().unwrap().body.cleanup_line.is_none(), "it stands in the suggestion line's place");
    let unless = format!("Deleted {} unless pinned", day(DUE));
    assert_eq!(row_going(&r.scene, "Weekly update, Sep 18").as_deref(), Some(unless.as_str()));
    assert_eq!(row_going(&r.scene, "Where the 502s come from"), None);
    let pin = r.scene.library.as_ref().unwrap().body.rows.iter().find(|x| x.title.as_deref() == Some("Weekly update, Sep 18")).unwrap().buttons.iter().find(|b| b.label == "Pin").cloned().expect("a going row's Pin, beside Copy link");
    assert_eq!((pin.action.clone(), pin.style), (Action::PinRow(rec("weekly"), true), Style::Plain));

    r.lib(Action::PinRow(rec("weekly"), true));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Pin(vec![rec("weekly")], true))));
    assert_eq!(going_line(&r.scene).unwrap(), format!("One video will be deleted on {} — nobody has watched it in 30 days.", day(DUE)));
    assert_eq!(row_going(&r.scene, "Weekly update, Sep 18"), None, "pinned: its warning goes at once");
    r.lib(Action::PinRow(rec("design"), true));
    assert_eq!(going_line(&r.scene), None, "the line goes with the last one");
    assert!(r.scene.library.as_ref().unwrap().body.cleanup_line.is_some(), "and the suggestions' line is back in its place");
}

#[test]
fn review_shows_whats_going_first_then_the_suggestions() {
    let mut w = library(facts(30, two_going(), None));
    w.session.fields.library.cleanup = true;
    let r = Run::new(w);
    let v = r.scene.library.as_ref().unwrap().body.cleanup.clone().unwrap();
    assert_eq!(v.going.len(), 1, "one date, one heading");
    let g = &v.going[0];
    assert_eq!(g.heading, format!("Deleting on {}", day(DUE)));
    assert_eq!(g.line.as_deref(), Some("Nobody has watched these in 30 days. Pin one to keep it; a watch before then keeps it too."));
    let titles: Vec<_> = g.rows.iter().map(|x| x.title.clone().unwrap_or_default()).collect();
    assert_eq!(titles, ["Weekly update, Sep 18", "Design review: settings sheet"]);
    assert!(g.rows[0].meta.ends_with("640 MB · Watched twice, the last 31 days ago"), "no cost: it's going anyway");
    assert_eq!(g.rows[0].going, Some(format!("Deleted {} unless pinned", day(DUE))), "amber, after the facts");
    let labels: Vec<_> = g.rows[0].buttons.iter().map(|b| b.label.as_str()).collect();
    assert_eq!(labels, ["Open page", "Pin"], "no Delete: it's going anyway");
    assert_eq!(v.suggest_heading.as_deref(), Some("Might be worth deleting"));
    assert_eq!(v.rows.len(), 1, "the suggestions follow");
    // With nothing going there is no heading over the suggestions.
    let mut w = library(facts(30, vec![], None));
    w.session.fields.library.cleanup = true;
    let v = Run::new(w).scene.library.as_ref().unwrap().body.cleanup.clone().unwrap();
    assert!(v.going.is_empty() && v.suggest_heading.is_none());
}

#[test]
fn the_popover_says_it_too_with_pin_both_review_and_not_now() {
    let mut r = Run::new(popover(facts(30, two_going(), None)));
    let (id, line, buttons) = banner(&r.scene).expect("the banner");
    assert_eq!(id, "autoclean-warning");
    assert_eq!(line, format!("Two videos will be deleted on {} — nobody has watched them in 30 days.", day(DUE)));
    assert_eq!(buttons, ["Not now", "Review…", "Pin both"]);
    let b = r.scene.popover.as_ref().unwrap().banner.clone().unwrap();
    assert_eq!(b.tone, shroom_core::scene::Tone::Amber);
    assert_eq!(b.buttons[2].style, Style::Filled);
    assert_eq!(r.scene.popover.as_ref().unwrap().ret, Some(Action::Start), "a banner's button is never the default (law 8)");

    r.pop(Action::PinGoing);
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Pin(vec![rec("weekly"), rec("design")], true))), "every video it counts, in one tap");
    let (_, line, buttons) = banner(&r.scene).unwrap();
    assert_eq!(line, format!("Pinned both. Nothing will be deleted on {}.", day(DUE)));
    assert!(buttons.is_empty());
    r.send(Event::Library(LibraryNews::Pinned(vec![rec("weekly"), rec("design")], true)));
    r.act(Action::Dismiss);
    r.click();
    assert!(banner(&r.scene).is_none(), "said while the popover stays open; gone the next time");
}

#[test]
fn pin_both_reads_its_count() {
    let one = Run::new(popover(facts(30, vec![going("weekly", 640_000_000, 2, 31)], None)));
    assert_eq!(banner(&one.scene).unwrap().2, ["Not now", "Review…", "Pin it"]);
    assert!(banner(&one.scene).unwrap().1.starts_with("One video will be deleted on"));
    let three = Run::new(popover(facts(30, vec![going("weekly", 640_000_000, 2, 31), going("design", 1_100_000_000, 0, 33), going("r502", 300_000_000, 0, 40)], None)));
    assert_eq!(banner(&three.scene).unwrap().2[2], "Pin all three");
}

#[test]
fn review_opens_the_library_on_whats_going() {
    let mut r = Run::new(popover(facts(30, two_going(), None)));
    r.pop(Action::ReviewGoing);
    assert_eq!(r.arm.edge, Some(("autoclean-warning", "library-cleanup")));
    assert!(r.ids().contains(&"library-cleanup"));
    assert!(!r.scene.library.as_ref().unwrap().body.cleanup.as_ref().unwrap().going.is_empty());
}

#[test]
fn not_now_puts_the_banner_away_on_this_mac_until_the_list_changes() {
    let mut r = Run::new(popover(facts(30, two_going(), None)));
    r.pop(Action::DismissGoing);
    assert_eq!(r.arm.edge, Some(("autoclean-warning", "config")));
    assert!(banner(&r.scene).is_none());
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.going_dismissed == ["design", "weekly"])), "remembered on this Mac");
    r.send(Event::Library(LibraryNews::Suggested(facts(30, two_going(), None))));
    assert!(banner(&r.scene).is_none(), "the same list: still away");
    let mut more = two_going();
    more.push(going("r502", 300_000_000, 0, 40));
    r.send(Event::Library(LibraryNews::Suggested(facts(30, more, None))));
    assert!(banner(&r.scene).is_some(), "a new one: back");
    // The Library's line stays whatever the popover's banner did.
    let mut w = library(facts(30, two_going(), None));
    w.prefs.going_dismissed = vec!["design".into(), "weekly".into()];
    assert!(going_line(&Run::new(w).scene).is_some());
}

#[test]
fn the_banner_ranks_under_a_saved_take_and_over_an_update_and_the_ai_offer() {
    let mut w = popover(facts(30, two_going(), None));
    w.space.ai = Some(false);
    assert_eq!(banner(&Run::new(w.clone()).scene).unwrap().0, "autoclean-warning", "over the AI offer");
    w.saved.push(saved(SavedCause::Interrupted, 272_000));
    assert_eq!(banner(&Run::new(w).scene).unwrap().0, "recovered-take", "a saved take first");
}

#[test]
fn once_a_round_has_gone_the_library_and_the_popover_say_so_until_ok() {
    let last = Cleaned { at: Wall(NOW - DAY / 2), recs: vec![rec("old1"), rec("old2")], bytes: 1_700_000_000 };
    let mut w = popover(facts(30, vec![], Some(last.clone())));
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    let mut r = Run::new(w);
    let said = "Auto-cleanup deleted two videos nobody had watched in 30 days, freeing 1.7 GB.";
    assert_eq!(banner(&r.scene).unwrap(), ("autoclean-warning", said.to_string(), vec!["OK".to_string()]));
    let l = r.scene.library.as_ref().unwrap().body.cleaned_line.clone().unwrap();
    assert_eq!((l.line.as_str(), l.buttons[0].label.as_str()), (said, "OK"));
    r.pop(Action::CleanedOk);
    assert!(banner(&r.scene).is_none() && r.scene.library.as_ref().unwrap().body.cleaned_line.is_none(), "OK in either clears both");
    assert!(r.has(|e| matches!(e, Effect::Persist(PersistOp::Prefs(p)) if p.cleaned_seen == Some(Wall(NOW - DAY / 2)))));
    // One reads its own words.
    let one = Run::new(library(facts(30, vec![], Some(Cleaned { at: Wall(NOW), recs: vec![rec("old1")], bytes: 640_000_000 }))));
    assert_eq!(one.scene.library.as_ref().unwrap().body.cleaned_line.as_ref().unwrap().line, "Auto-cleanup deleted one video nobody had watched in 30 days, freeing 640 MB.");
}

#[test]
fn the_setting_mac_sets_it_and_turning_it_on_asks_when_videos_already_qualify() {
    let mut r = Run::new(videos_tab(facts(0, vec![], None)));
    let a = videos(&r.scene).autoclean.clone().expect("Delete videos nobody watches");
    assert_eq!(a.label, "Delete videos nobody watches");
    assert_eq!(a.hint, "Counted from the last time someone watched it, or from when it was shared. Pinned videos are never deleted, and the Library says which ones are going a few days before.");
    let choices: Vec<_> = a.choices.iter().map(|c| (c.label.as_str(), c.selected, c.enabled)).collect();
    assert_eq!(choices, [("Off", true, true), ("After 15 days", false, true), ("After 30 days", false, true), ("After 60 days", false, true), ("After a year", false, true)], "off as it ships");

    r.set(Action::SetAutoclean(30));
    assert_eq!(r.arm.edge, Some(("settings-videos", "autoclean-confirm")));
    assert!(r.effects.is_empty(), "nothing set before the question's answered");
    let sheet = r.scene.settings.as_ref().unwrap().body.sheet.clone().unwrap();
    assert_eq!(sheet.id, "autoclean-confirm");
    assert_eq!(sheet.title, "Delete videos nobody watches for 30 days?");
    assert_eq!(sheet.lines, [format!("Three videos already qualify. They'll be deleted on {} unless someone watches them or you pin them — the Library shows which. Their links will say the videos are gone.", day(DUE))]);
    let labels: Vec<_> = sheet.buttons.iter().map(|b| (b.label.as_str(), b.style)).collect();
    assert_eq!(labels, [("Cancel", Style::Plain), ("Turn on", Style::RedOutline)]);
    assert_eq!(r.scene.settings.as_ref().unwrap().ret, None, "no Return default (law 9)");

    r.set(Action::Back);
    assert_eq!(r.arm.edge, Some(("autoclean-confirm", "settings-videos")));
    assert!(r.effects.is_empty());

    r.set(Action::SetAutoclean(30));
    r.set(Action::ConfirmAutoclean);
    assert_eq!(r.arm.edge, Some(("autoclean-confirm", "settings-videos")));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAutoclean(30))));
    let a = videos(&r.scene).autoclean.clone().unwrap();
    assert_eq!(a.note.as_deref(), Some("Turning on — a few seconds."), "the first time gives the page its schedule");
    assert!(a.choices.iter().all(|c| !c.enabled), "greyed while it's set");
    r.send(Event::Cloud(CloudNews::Autoclean(30)));
    let a = videos(&r.scene).autoclean.clone().unwrap();
    assert_eq!(a.note, None);
    assert!(a.choices.iter().any(|c| c.label == "After 30 days" && c.selected));
}

#[test]
fn a_year_is_a_choice_and_its_lines_say_a_year() {
    // N77 (Bagrat, 2026-10-25: add a year): 365 days, said as the landing says it.
    let mut s = facts(0, vec![], None);
    s.idle.push(Idle { rec: rec("r502"), quiet_days: 400 });
    let mut r = Run::new(videos_tab(s));
    r.set(Action::SetAutoclean(365));
    let sheet = r.scene.settings.as_ref().unwrap().body.sheet.clone().expect("one already qualifies: it asks");
    assert_eq!(sheet.title, "Delete videos nobody watches for a year?");
    r.set(Action::ConfirmAutoclean);
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAutoclean(365))));
    r.send(Event::Cloud(CloudNews::Autoclean(365)));
    assert!(videos(&r.scene).autoclean.as_ref().unwrap().choices.iter().any(|c| c.label == "After a year" && c.selected));

    let mut w = videos_tab(facts(365, vec![], None));
    w.space.token = false;
    assert_eq!(videos(&Run::new(w).scene).autoclean.as_ref().unwrap().hint, "Your space deletes videos nobody has watched in a year. It's set on the Mac that set up your space. Pin a video in the Library to keep it.");

    let year = |n: u32| vec![going("weekly", 640_000_000, 2, n), going("design", 1_100_000_000, 0, n)];
    let lib = Run::new(library(facts(365, year(366), None)));
    assert_eq!(going_line(&lib.scene).unwrap(), format!("Two videos will be deleted on {} — nobody has watched them in a year.", day(DUE)));
    assert_eq!(banner(&Run::new(popover(facts(365, year(366), None))).scene).unwrap().1, format!("Two videos will be deleted on {} — nobody has watched them in a year.", day(DUE)));
    let mut w = library(facts(365, year(366), None));
    w.session.fields.library.cleanup = true;
    let v = Run::new(w).scene.library.as_ref().unwrap().body.cleanup.clone().unwrap();
    assert_eq!(v.going[0].line.as_deref(), Some("Nobody has watched these in a year. Pin one to keep it; a watch before then keeps it too."));
    let gone = Run::new(library(facts(365, vec![], Some(Cleaned { at: Wall(NOW), recs: vec![rec("old1")], bytes: 640_000_000 }))));
    assert_eq!(gone.scene.library.as_ref().unwrap().body.cleaned_line.as_ref().unwrap().line, "Auto-cleanup deleted one video nobody had watched in a year, freeing 640 MB.");
}

#[test]
fn a_longer_wait_off_or_nothing_qualifying_needs_no_asking() {
    let mut r = Run::new(videos_tab(facts(30, vec![], None)));
    r.set(Action::SetAutoclean(60));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAutoclean(60))), "longer: set at once");
    r.send(Event::Cloud(CloudNews::Autoclean(60)));
    r.set(Action::SetAutoclean(0));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAutoclean(0))), "off: at once");
    // Shorter, but none would be due: at once.
    let mut s = facts(60, vec![], None);
    s.idle.clear();
    let mut r = Run::new(videos_tab(s));
    r.set(Action::SetAutoclean(15));
    assert!(r.has(|e| *e == Effect::Cloud(CloudCmd::SetAutoclean(15))));
    assert!(r.scene.settings.as_ref().unwrap().body.sheet.is_none());
}

#[test]
fn a_setting_that_didnt_go_through_says_so_and_stays_as_it_was() {
    let mut r = Run::new(videos_tab(facts(30, vec![], None)));
    r.set(Action::SetAutoclean(60));
    r.send(Event::Failed { op: Op::Autoclean, class: FailClass::Transport });
    let a = videos(&r.scene).autoclean.clone().unwrap();
    assert_eq!(a.note.as_deref(), Some("Couldn't change that just now — try again in a moment."));
    assert!(a.choices.iter().any(|c| c.label == "After 30 days" && c.selected), "as the space has it");
}

#[test]
fn every_other_mac_reads_it_and_counting_off_greys_it() {
    let mut w = videos_tab(facts(30, vec![], None));
    w.space.token = false;
    let a = videos(&Run::new(w).scene).autoclean.clone().unwrap();
    assert_eq!(a.hint, "Your space deletes videos nobody has watched in 30 days. It's set on the Mac that set up your space. Pin a video in the Library to keep it.");
    assert!(a.choices.iter().all(|c| !c.enabled));
    assert!(a.choices.iter().any(|c| c.label == "After 30 days" && c.selected));

    let mut s = facts(30, vec![], None);
    s.counting = false;
    let a = videos(&Run::new(videos_tab(s)).scene).autoclean.clone().unwrap();
    assert_eq!(a.hint, "Needs Count plays");
    assert!(a.choices.iter().all(|c| !c.enabled));
}

#[test]
fn the_popover_reads_whats_going_at_most_once_an_hour() {
    let mut w = space(facts(30, vec![], None));
    w.suggestions = Fact::Unknown;
    let mut r = Run::new(w);
    r.click();
    assert!(r.has(|e| matches!(e, Effect::Library(LibCmd::Suggest { .. }))), "an unknown list is read as the popover opens");
    r.send(Event::Library(LibraryNews::Suggested(facts(30, two_going(), None))));
    r.act(Action::Dismiss);
    r.click();
    assert!(!r.has(|e| matches!(e, Effect::Library(LibCmd::Suggest { .. }))), "read moments ago: not again");
}

#[test]
fn the_autoclean_op_has_a_deadline() {
    assert_eq!(Op::Autoclean.deadline_ms(), 60_000);
}

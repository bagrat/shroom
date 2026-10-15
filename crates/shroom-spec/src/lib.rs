//! The machine (`spec/machine.json`, DECISIONS B133) as Rust types: every state, its
//! said-lines and actions, the edges, the rules that hold across a group of states, the laws
//! and the ledger. Tests only — the product never reads it.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SaidLine {
    pub id: String,
    pub text: String,
    pub role: String,
    /// `ratified`, `provisional` or `draft`.
    #[serde(rename = "copy")]
    pub copy_status: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Action {
    pub label: String,
    pub kind: String,
    pub return_default: bool,
    #[serde(default)]
    pub goes_to: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// One edge out of a state, as its state sees it.
#[derive(Debug, Clone)]
pub struct Transition {
    pub to: String,
    pub trigger: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct State {
    /// Its key in `states`.
    #[serde(skip)]
    pub id: String,
    pub region: u32,
    pub kind: String,
    #[serde(default)]
    pub parent: Option<String>,
    pub surface: String,
    /// `ratified`, `provisional` (awaiting ratification), or `structure-only`.
    pub status: String,
    /// Its screen board on the shroom UI canvas.
    #[serde(default)]
    pub board: Option<String>,
    pub summary: String,
    #[serde(rename = "entry")]
    pub entry_conditions: String,
    #[serde(rename = "said")]
    pub said_lines: Vec<SaidLine>,
    pub actions: Vec<Action>,
    #[serde(rename = "cites", default)]
    pub decided_by: Vec<String>,
    /// The edges out of it (filled from `edges` on load).
    #[serde(skip)]
    pub transitions: Vec<Transition>,
}

impl State {
    pub fn is_provisional(&self) -> bool {
        self.status == "provisional"
    }
    pub fn return_default(&self) -> Option<&Action> {
        self.actions.iter().find(|a| a.return_default)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: String,
    pub trigger: String,
    pub status: String,
    pub cites: Vec<String>,
    /// Where it was drawn first: `map`, `fixture`, `core-arm` (an `Arm.edge`), `core-run`,
    /// `canvas` (a board, before any code).
    pub source: Vec<String>,
    /// The events the core was seen to move on.
    pub on: Vec<String>,
}

/// A move that holds for a whole group of states, written once (B133, Bagrat's (b)).
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    pub label: String,
    /// State ids, `@group`s, `*` (any state), or the tray's `closed` / a banner's `none`.
    pub from: Vec<String>,
    pub to: Vec<String>,
    /// Event patterns (`Spec::events` names the shorthands).
    pub on: Vec<String>,
    /// Also `to → from`.
    #[serde(default)]
    pub reverse: bool,
    /// Only between a state and its own faces, or two faces of one state.
    #[serde(default)]
    pub family: bool,
    pub kind: String,
    pub says: String,
    pub cites: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub states: Vec<String>,
    pub cites: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Law {
    pub n: u32,
    pub title: String,
    pub rule: String,
    pub invariants: Vec<String>,
    pub cites: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LedgerRow {
    pub id: String,
    pub title: String,
    pub decision: String,
    pub status: String,
    pub date: String,
    pub amends: Vec<String>,
    pub amended_by: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegionInfo {
    pub id: u32,
    pub title: String,
}

#[derive(Debug, Deserialize)]
struct Raw {
    events: HashMap<String, serde_json::Value>,
    laws: Vec<Law>,
    ledger: Vec<LedgerRow>,
    regions: Vec<RegionInfo>,
    groups: Vec<Group>,
    states: serde_json::Map<String, serde_json::Value>,
    edges: Vec<Edge>,
    rules: Vec<Rule>,
}

/// The machine, loaded once.
pub struct Spec {
    pub regions: Vec<RegionInfo>,
    states: Vec<State>,
    pub edges: Vec<Edge>,
    pub rules: Vec<Rule>,
    pub groups: Vec<Group>,
    pub laws: Vec<Law>,
    pub ledger: Vec<LedgerRow>,
    events: HashMap<String, Vec<String>>,
}

/// The tray's own resting place when nothing of shroom's is open, and a popover with no
/// banner: the two ends a rule may name that aren't states.
pub const NOT_STATES: &[&str] = &["closed", "none"];

impl Spec {
    pub fn load() -> Spec {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/machine.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let raw: Raw = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut states: Vec<State> = raw
            .states
            .into_iter()
            .map(|(id, v)| {
                let mut s: State = serde_json::from_value(v).unwrap_or_else(|e| panic!("{}: state {id}: {e}", path.display()));
                s.id = id;
                s
            })
            .collect();
        states.sort_by_key(|s| s.region);
        for s in &mut states {
            s.transitions = raw
                .edges
                .iter()
                .filter(|e| e.from == s.id)
                .map(|e| Transition { to: e.to.clone(), trigger: if e.trigger.is_empty() { e.label.clone() } else { e.trigger.clone() } })
                .collect();
        }
        let events = raw
            .events
            .into_iter()
            .filter_map(|(k, v)| serde_json::from_value::<Vec<String>>(v).ok().map(|list| (k, list)))
            .collect();
        Spec { regions: raw.regions, states, edges: raw.edges, rules: raw.rules, groups: raw.groups, laws: raw.laws, ledger: raw.ledger, events }
    }

    /// Every state, region by region.
    pub fn states(&self) -> impl Iterator<Item = &State> {
        self.states.iter()
    }

    /// One region's states.
    pub fn region(&self, n: u32) -> impl Iterator<Item = &State> {
        self.states.iter().filter(move |s| s.region == n)
    }

    /// States by id (one each).
    pub fn by_id(&self) -> HashMap<&str, Vec<&State>> {
        self.states.iter().map(|s| (s.id.as_str(), vec![s])).collect()
    }

    pub fn state(&self, id: &str) -> Option<&State> {
        self.states.iter().find(|s| s.id == id)
    }

    pub fn law(&self, n: u32) -> Option<&Law> {
        self.laws.iter().find(|l| l.n == n)
    }

    pub fn edge(&self, from: &str, to: &str) -> Option<&Edge> {
        self.edges.iter().find(|e| e.from == from && e.to == to)
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// A state and every state it is a face of, nearest first.
    pub fn ancestors<'a>(&'a self, id: &'a str) -> Vec<&'a str> {
        let mut out = vec![id];
        let mut at = id;
        while let Some(p) = self.state(at).and_then(|s| s.parent.as_deref()) {
            out.push(p);
            at = p;
        }
        out
    }

    /// A rule's ends, `@group`s expanded.
    pub fn expand<'a>(&'a self, ends: &'a [String]) -> Vec<&'a str> {
        let mut out = Vec::new();
        for e in ends {
            match e.strip_prefix('@') {
                Some(g) => out.extend(self.group(g).unwrap_or_else(|| panic!("no group {g}")).states.iter().map(String::as_str)),
                None => out.push(e.as_str()),
            }
        }
        out
    }

    /// Whether an event, by its kind (`Act(Restart)`, `Cloud(Swept)`, `TrayClicked`…), is one
    /// of a rule's patterns.
    pub fn event_matches(&self, patterns: &[String], on: &str) -> bool {
        patterns.iter().any(|p| {
            let expanded: Vec<&str> = match self.events.get(p) {
                Some(list) => list.iter().map(String::as_str).collect(),
                None => vec![p.as_str()],
            };
            expanded.iter().any(|q| *q == "*" || on == *q || on.strip_prefix(q).is_some_and(|rest| rest.starts_with('(')))
        })
    }

    fn family(&self, from: &str, to: &str) -> bool {
        let (af, at) = (self.ancestors(from), self.ancestors(to));
        af.contains(&to) || at.contains(&from) || (af.len() > 1 && at.len() > 1 && af[1] == at[1])
    }

    /// The rule that allows a move, if one does.
    pub fn rule_for(&self, from: &str, to: &str, on: &str) -> Option<&Rule> {
        let has = |ends: &[String], s: &str| ends.iter().any(|e| e == "*") || self.expand(ends).contains(&s);
        self.rules.iter().find(|r| {
            self.event_matches(&r.on, on)
                && if r.family {
                    self.family(from, to)
                } else {
                    (has(&r.from, from) && has(&r.to, to)) || (r.reverse && has(&r.to, from) && has(&r.from, to))
                }
        })
    }

    /// Whether the machine has a move: an edge between the two states (or a state each is a
    /// face of), or a rule that holds for it. The answer names which.
    pub fn listed(&self, from: &str, to: &str, on: &str) -> Option<String> {
        if from == to {
            return Some(String::new());
        }
        if self.edge(from, to).is_some() {
            return Some(format!("edge {from} → {to}"));
        }
        if let Some(r) = self.rule_for(from, to, on) {
            return Some(format!("rule {}", r.id));
        }
        for f in self.ancestors(from) {
            for t in self.ancestors(to) {
                if self.edge(f, t).is_some() {
                    return Some(format!("edge {f} → {t}"));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_loads_and_holds_what_it_should() {
        let spec = Spec::load();
        assert_eq!(spec.regions.len(), 6);
        assert!(spec.states().count() >= 139);
        assert_eq!(spec.laws.len(), 16);
        assert!(spec.state("paused").and_then(|s| s.return_default()).is_some_and(|a| a.label == "Resume"));
        assert!(spec.state("config").and_then(|s| s.return_default()).is_some_and(|a| a.label == "Start recording"));
    }

    #[test]
    fn every_id_is_one_kebab_case_name_and_every_end_is_a_state() {
        let spec = Spec::load();
        let kebab = |s: &str| !s.is_empty() && s.split('-').all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        for s in spec.states() {
            assert!(kebab(&s.id), "{} isn't kebab-case", s.id);
            if let Some(p) = &s.parent {
                assert!(spec.state(p).is_some(), "{}'s parent {p} isn't a state", s.id);
            }
        }
        for e in &spec.edges {
            assert!(spec.state(&e.from).is_some() && spec.state(&e.to).is_some(), "edge {} → {} has an end that isn't a state", e.from, e.to);
        }
        for g in &spec.groups {
            for s in &g.states {
                assert!(spec.state(s).is_some(), "group {} names {s}", g.id);
            }
        }
        for r in &spec.rules {
            for s in spec.expand(&r.from).into_iter().chain(spec.expand(&r.to)) {
                assert!(s == "*" || NOT_STATES.contains(&s) || spec.state(s).is_some(), "rule {} names {s}", r.id);
            }
        }
        for s in spec.states() {
            for a in &s.actions {
                if let Some(g) = &a.goes_to {
                    assert!(NOT_STATES.contains(&g.as_str()) || spec.state(g).is_some(), "{}: {} goes to {g:?}, not a state", s.id, a.label);
                }
            }
        }
        let ids: Vec<_> = spec.states().flat_map(|s| s.said_lines.iter().map(|l| l.id.as_str())).collect();
        let unique: std::collections::BTreeSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "two said-lines share an id");
    }

    #[test]
    fn a_rule_expands_into_its_moves() {
        let spec = Spec::load();
        assert_eq!(spec.rule_for("token", "wizard-offline", "Os(NetPath)").map(|r| r.id.as_str()), Some("offline"));
        assert_eq!(spec.rule_for("wizard-offline", "card", "Os(NetPath)").map(|r| r.id.as_str()), Some("offline"));
        assert_eq!(spec.rule_for("account", "idle-unprovisioned", "WindowClosed").map(|r| r.id.as_str()), Some("close-window"));
        assert!(spec.rule_for("account", "recording", "WindowClosed").is_none());
        assert!(spec.listed("paused", "restart-confirm", "Act(Restart)").is_some());
        assert!(spec.listed("paused", "countdown", "Act(Restart)").is_none(), "Restart asks first (B134)");
    }

    #[test]
    fn destructive_actions_are_never_the_return_default() {
        // Law 9, as the machine records it.
        let spec = Spec::load();
        for s in spec.states() {
            for a in &s.actions {
                if a.kind == "destructive" || a.kind == "red-text" {
                    assert!(!a.return_default, "{}: {} is destructive and a Return default", s.id, a.label);
                }
            }
        }
    }

    #[test]
    fn every_surface_has_at_most_one_return_default() {
        // Law 8, as the machine records it.
        let spec = Spec::load();
        for s in spec.states() {
            let defaults: Vec<_> = s.actions.iter().filter(|a| a.return_default).map(|a| a.label.as_str()).collect();
            assert!(defaults.len() <= 1, "{}: several Return defaults {defaults:?}", s.id);
        }
    }
}

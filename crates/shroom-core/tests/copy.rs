//! Every line in copy.rs, with the spec's example arguments, is a said-line (or an action
//! label) of its state in spec/r*.json — verbatim (TESTING-PLAN §2.4).

use shroom_spec::Spec;

#[test]
fn every_line_is_the_specs() {
    let spec = Spec::load();
    let by_id = spec.by_id();
    let mut missing = Vec::new();
    for (state, text) in shroom_core::copy::registry() {
        let states = by_id.get(state).unwrap_or_else(|| panic!("no spec state {state:?}"));
        let found = states.iter().any(|s| s.said_lines.iter().any(|l| l.text == text) || s.actions.iter().any(|a| a.label == text));
        if !found {
            missing.push(format!("{state}: {text:?}"));
        }
    }
    assert!(missing.is_empty(), "not verbatim in the spec:\n{}", missing.join("\n"));
}

#[test]
fn counts_and_lengths() {
    assert_eq!(shroom_core::copy::count_word(2), "two");
    assert_eq!(shroom_core::copy::count_word(14), "14");
    assert_eq!(shroom_core::copy::length(272_000), "4:32");
    assert_eq!(shroom_core::copy::length(3_729_000), "1:02:09");
}

#[test]
fn the_apps_usage_strings_are_the_ratified_ones() {
    let plist = include_str!("../../../app/src-tauri/Info.plist");
    assert!(plist.contains(&format!("<string>{}</string>", shroom_core::copy::mic_purpose())));
    assert!(plist.contains(&format!("<string>{}</string>", shroom_core::copy::camera_purpose())));
}

/// Parts of lines, not lines: the counts, the lengths, the dates, the name guard, and a
/// row composed of a label and a said-line.
const NOT_SAID: &[(&str, &str)] = &[
    ("count_word", "a count inside a line (law 16)"),
    ("length", "a take's length inside a line"),
    ("day", "a date inside a line"),
    ("row_meta", "a Library row's date · length, both parts"),
    ("filter_name", "the link-name guard: it returns the name typed, not a line"),
    ("invited_row", "composed: the row's label, a dash, and invited_fact's said-line"),
    ("size", "a size inside a line (N68): “11 GB”, “780 MB”"),
    ("money", "money inside a line (N68): “17¢”"),
    ("wait", "auto-cleanup's wait inside a line (N77): “30 days”, “a year”"),
];

/// Every line copy.rs ships is a said-line of spec/machine.json (through the registry,
/// checked above) or named here as not a line (B133, phase 2).
#[test]
fn every_shipped_line_is_a_said_line_or_named_not_one() {
    let src = include_str!("../src/copy.rs");
    let body = src.split("#[cfg(test)]").next().unwrap();
    let registry = &body[body.find("pub fn registry()").expect("copy.rs has its registry")..];
    let mut lines = Vec::new();
    for (at, _) in body.match_indices("pub fn ") {
        let rest = &body[at + 7..];
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let sig = &rest[..rest.find('{').unwrap_or(rest.len())];
        if sig.contains("-> String") && name != "registry" {
            lines.push(name);
        }
    }
    assert!(lines.len() > 200, "the scan found only {} lines — has copy.rs's shape changed?", lines.len());
    let unsaid: Vec<_> = lines
        .iter()
        .filter(|f| !registry.contains(&format!("{f}(")) && !NOT_SAID.iter().any(|(n, _)| n == f))
        .collect();
    assert!(unsaid.is_empty(), "lines copy.rs ships that are neither a said-line (the registry) nor named not one: {unsaid:?}");
    for (n, _) in NOT_SAID {
        assert!(lines.contains(&n.to_string()), "NOT_SAID names {n}, which copy.rs doesn't have");
    }
}

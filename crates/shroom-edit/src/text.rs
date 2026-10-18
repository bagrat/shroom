//! Word normalisation and fuzzy word likeness.

/// Lowercase, letters/digits/apostrophes only ("Shroom," → "shroom").
pub fn norm(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .trim_matches('\'')
        .to_string()
}

/// Words a speaker drops in around a restart that carry no content ("so", "um", "okay").
pub fn is_filler(w: &str) -> bool {
    is_hesitation(w)
        || matches!(
            w,
            "oh" | "okay"
                | "ok"
                | "so"
                | "alright"
                | "right"
                | "again"
                | "sorry"
                | "well"
                | "anyway"
        )
}

/// Pure hesitation sounds ("um", "uh"): cut with a cue they follow; "so" and "okay" stay.
pub fn is_hesitation(w: &str) -> bool {
    matches!(
        w,
        "um" | "uh" | "erm" | "er" | "ah" | "hmm" | "hm" | "hem" | "mm"
    )
}

/// A transcript word that ends a sentence ("shroom." / "right?").
pub fn ends_sentence(raw: &str) -> bool {
    raw.trim_end().ends_with(['.', '?', '!'])
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// How alike two normalised words are: 1 the same, 0.7 a near-miss of a longer word
/// ("recording" / "recordings"), 0 otherwise.
pub fn likeness(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    let longest = a.chars().count().max(b.chars().count());
    if longest >= 5 && levenshtein(a, b) * 5 <= longest {
        0.7
    } else {
        0.0
    }
}

/// A rough sound-alike key, so the code word survives the transcriber's spelling
/// ("Shroom", "Schroom", "shrewm", "shrum" all key to "shrum").
pub fn sound_key(w: &str) -> String {
    let mut s = w.replace("sch", "sh").replace("ph", "f");
    for (from, to) in [
        ("oo", "u"),
        ("ew", "u"),
        ("ou", "u"),
        ("ue", "u"),
        ("ck", "k"),
        ("c", "k"),
    ] {
        s = s.replace(from, to);
    }
    let mut out = String::new();
    for c in s.chars() {
        if !out.ends_with(c) {
            out.push(c);
        }
    }
    out
}

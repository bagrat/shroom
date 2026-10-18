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

/// The sentence ended as said, not trailing off: a full stop, a question or an exclamation,
/// never an ellipsis ("you will..." is a sentence broken off), and on a word a sentence can
/// end on. Whisper puts a full stop wherever a speaker stops ("and you will."), so the word
/// counts as much as the mark.
pub fn ends_cleanly(raw: &str) -> bool {
    let t = raw.trim_end();
    ends_sentence(t) && !t.ends_with("...") && !t.ends_with('…') && !dangles(&norm(t))
}

/// A sentence, its words as transcribed, ended as said ([`ends_cleanly`]), and not on "is",
/// "are", "was" or "were" left hanging: one says what something is after a pronoun ("that's
/// what it is.") or under a question word in the same clause ("where your files are."), and
/// "…and your link is." says neither, so it was broken off.
pub fn ends_whole(said: &[&str]) -> bool {
    let Some((last, before)) = said.split_last() else {
        return false;
    };
    if !ends_cleanly(last) || !matches!(norm(last).as_str(), "is" | "are" | "was" | "were") {
        return ends_cleanly(last);
    }
    if before.last().is_some_and(|w| says_what(&norm(w))) {
        return true;
    }
    // The clause it ends: back to a comma, or to a word joining two.
    let clause = before
        .iter()
        .rev()
        .take_while(|w| {
            !w.trim_end().ends_with([',', ';', ':'])
                && !matches!(norm(w).as_str(), "and" | "but" | "or" | "so" | "because")
        })
        .map(|w| norm(w))
        .collect::<Vec<_>>();
    clause.iter().any(|w| {
        matches!(
            w.as_str(),
            "what"
                | "where"
                | "how"
                | "who"
                | "whom"
                | "which"
                | "whatever"
                | "wherever"
                | "whoever"
        )
    })
}

/// A word that can stand before "is" at a sentence's end ("it is.", "here you are.").
fn says_what(w: &str) -> bool {
    matches!(
        w,
        "it" | "that"
            | "this"
            | "these"
            | "those"
            | "there"
            | "here"
            | "what"
            | "who"
            | "which"
            | "he"
            | "she"
            | "they"
            | "we"
            | "you"
            | "i"
    )
}

/// A word an English sentence doesn't end on: an article, a joining word, "to" or "of", a
/// modal ("you will"), a possessive ("your"). Words that often do end one ("it", "that",
/// "on", "in", "is") aren't here: a sentence wrongly read as broken off loses words it
/// kept ("is" left hanging is [`ends_whole`]'s, with the word before it).
pub fn dangles(w: &str) -> bool {
    matches!(
        w,
        "a" | "an"
            | "the"
            | "and"
            | "or"
            | "but"
            | "because"
            | "than"
            | "to"
            | "of"
            | "from"
            | "into"
            | "with"
            | "will"
            | "would"
            | "could"
            | "should"
            | "shall"
            | "must"
            | "might"
            | "can"
            | "my"
            | "your"
            | "our"
            | "their"
    )
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

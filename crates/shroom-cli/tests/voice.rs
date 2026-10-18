//! Every line the command says is in `src/say.rs` (spec N62, DECISIONS B215), where Bagrat
//! read and ratified them with N62: no other file of the command writes a sentence of its
//! own. A sentence, here: a string with at least three words in it. Tests aside.

/// The string literals on one line of Rust: what's between unescaped quotes.
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur: Option<String> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (&mut cur, c) {
            (None, '/') if chars.peek() == Some(&'/') => break,
            (None, '\'') => {
                // A char literal ('"', '\''), skipped whole; a lifetime has no closing quote.
                let rest: String = chars.clone().take(3).collect();
                if rest.starts_with("\\") {
                    chars.nth(2);
                } else if rest.chars().nth(1) == Some('\'') {
                    chars.nth(1);
                }
            }
            (None, '"') => cur = Some(String::new()),
            (Some(s), '\\') => {
                if let Some(n) = chars.next() {
                    s.push(n);
                }
            }
            (Some(s), '"') => {
                out.push(std::mem::take(s));
                cur = None;
            }
            (Some(s), c) => s.push(c),
            _ => {}
        }
    }
    out
}

/// Three or more words, each with a letter in it: something a person reads.
fn sentence(s: &str) -> bool {
    s.split_whitespace().filter(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 2).count() >= 3
}

#[test]
fn every_line_the_command_says_is_in_say_rs() {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut found = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(src).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "say.rs" || !name.ends_with(".rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        // The tests aside: they're the last thing in each file.
        let body = text.split("#[cfg(test)]").next().unwrap();
        for (n, line) in body.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for lit in literals(line).into_iter().filter(|l| sentence(l)) {
                found.push(format!("{name}:{}: {lit}", n + 1));
            }
        }
    }
    assert!(found.is_empty(), "lines said outside src/say.rs:\n{}", found.join("\n"));
}

#[test]
fn the_reader_reads_what_a_line_says() {
    assert_eq!(literals(r#"Fail::Usage("Couldn't read the list.".into()) // "a comment""#), ["Couldn't read the list."]);
    assert_eq!(literals(r#"let q = '"'; let s = "say \"hi\" there";"#), [r#"say "hi" there"#]);
    assert!(sentence("Nothing was deleted yet.") && !sentence("{:<16} {} {}") && !sentence("record.start"));
}

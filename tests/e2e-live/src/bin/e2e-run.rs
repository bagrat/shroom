//! The pre-merge e2e run's two ends (TESTING-PLAN D3), around the app on a hosted Mac:
//!
//!   e2e-run open <token-file> <space-id-file>
//!       the run's id made and written to <space-id-file> (where the app's `--e2e` reads
//!       it: `~/.shroom-dev/e2e-space-id`), the pin, crashed runs' leftovers reaped, and
//!       the run's own token — the product's two powers, two hours — written to
//!       <token-file> (mode 600, a new file) for the app's wizard to paste. Never printed.
//!   e2e-run close [<space-id-file>]
//!       the run's own space (`v-<run>`, `shroom-space-<run>`) and token deleted, crashed
//!       runs' leftovers reaped. Another run going on the account is never touched, so
//!       runs side by side need no lock (DECISIONS B131).
//!
//! Needs SHROOM_E2E=1 and the account token and id (see `config`).

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

/// A new file only this user reads, with `text` in it.
fn write_new(path: &str, text: &str) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path).map_err(|e| format!("{path}: {e}"))?;
    f.write_all(text.as_bytes()).map_err(|e| format!("{path}: {e}"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(cfg) = shroom_e2e_live::config() else {
        eprintln!("e2e-run: set SHROOM_E2E=1");
        std::process::exit(64);
    };
    let result = match (args.get(1).map(String::as_str), args.get(2), args.get(3)) {
        (Some("open"), Some(token_file), Some(space_file)) => {
            // The id is written before anything of the run exists, so a close always finds it.
            let run = shroom_e2e_live::new_run_id();
            write_new(space_file, &format!("{run}\n")).and_then(|()| shroom_e2e_live::open(&cfg, &run)).and_then(|o| {
                write_new(token_file, &o.value)?;
                Ok(format!("the account is pinned; run {run} has its own space and its token is in its file"))
            })
        }
        (Some("close"), space_file, None) => {
            let run = space_file.and_then(|f| std::fs::read_to_string(f).ok()).map(|s| s.trim().to_string()).filter(|r| shroom_cloud::wire::run_started(r).is_some());
            shroom_e2e_live::close(&cfg, run.as_deref()).map(|gone| format!("closed{}: {}", run.map(|r| format!(" run {r}")).unwrap_or_default(), if gone.is_empty() { "nothing to delete".to_string() } else { gone.join(", ") }))
        }
        _ => Err("usage: e2e-run open <token-file> <space-id-file> | close [<space-id-file>]".to_string()),
    };
    match result {
        Ok(said) => println!("e2e-run: {said}"),
        Err(e) => {
            eprintln!("e2e-run: {e}");
            std::process::exit(1);
        }
    }
}

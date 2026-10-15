//! The heartbeat's cloud half (TESTING-PLAN §5): a space for the dev app on the dedicated
//! test account (D3). Its resources are named `shroom-test-…` and the client is fenced to
//! that prefix.
//!
//!   shroom-dev dev-space up <worker.js> <hls.min.js>         → a space for the dev app on the e2e account:
//!                                                              writes ~/.shroom-dev/space.json, primed.json, token.json
//!   shroom-dev dev-space down                                → deletes that run's space and token, removes those files
//!
//! `dev-space` (the heartbeat's) runs on the dedicated test account (TESTING-PLAN D3): a
//! run of the e2e harness's (`tests/e2e-live`) — pinned, its own run id, wired with a
//! per-run token minted with the product's two powers — under `shroom-test-dev-<run>`, the
//! plain dev app's fence with the run's id, so it never touches an e2e run going beside it,
//! nor they it (DECISIONS B131). The dev app gets that token in its home's `token.json`,
//! marked as the harness's; down deletes the run's space and token, found by the run id its
//! `space.json` label ends in. (`up`, `down` and `sweep` on the main account retired with
//! main-account testing; `space-teardown`, which cleared a checkpoint's or the preview's
//! space off Bagrat's account, retired with the preview build — DECISIONS B148.)

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use shroom_cloud::{Cloud, Policy};

const PREFIX: &str = "shroom-test-";

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn dev_home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".shroom-dev")
}

fn write_600(path: &Path, value: &serde_json::Value) -> Res<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    f.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    Ok(())
}

/// Marks a `token.json` the harness wrote, so down removes only its own.
const HARNESS_MARK: &str = "e2e-run";

fn harness_token(home: &Path) -> Option<bool> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(home.join("token.json")).ok()?).ok()?;
    Some(v["harness"] == HARNESS_MARK)
}

fn dev_space_up(worker: &Path, hls_js: &Path) -> Res<()> {
    let home = dev_home();
    if harness_token(&home) == Some(false) {
        return Err("the dev home holds a pasted token.json — not the harness's to replace".into());
    }
    let cfg = shroom_e2e_live::credentials()?;
    // Its own run (B131): nothing another run has on the account is touched, nor waited on.
    let run = shroom_e2e_live::open(&cfg, &shroom_e2e_live::new_run_id())?;
    let wired = (|| -> Res<_> {
        let cloud = Cloud::new(&run.value, Policy::TestNamespace { prefix: PREFIX.into() });
        let name = format!("{PREFIX}dev-{}", run.run);
        // The product's own wiring, on the run token, with a fenced label and bucket.
        let module = std::fs::read_to_string(worker)?;
        let hls = std::fs::read(hls_js)?;
        let wired = shroom_cloud::wire::wire(
            &cloud,
            &shroom_cloud::wire::WireSpec {
                account_id: &cfg.account_id,
                token_id: &run.token_id,
                label: &name,
                bucket: &name,
                worker_module: &module,
                compatibility_date: "2026-09-01",
                assets: &[(".shroom/player/hls.min.js", &hls, "text/javascript")],
            },
        )?;
        Ok((name, wired))
    })();
    let (name, wired) = match wired {
        Ok(w) => w,
        Err(e) => {
            let _ = shroom_e2e_live::close(&cfg, Some(&run.run));
            return Err(e);
        }
    };
    std::fs::create_dir_all(&home)?;
    let identity = json!({
        "account_id": cfg.account_id, "label": name, "subdomain": wired.subdomain, "host": wired.host,
        "bucket": name, "role": "TokenHolder", "member_id": "m1", "owner_name": "You",
    });
    write_600(&home.join("token.json"), &json!({ "v": 1, "token": run.value, "account_id": cfg.account_id, "several": false, "harness": HARNESS_MARK }))?;
    write_600(&home.join("space.json"), &identity)?;
    write_600(&home.join("primed.json"), &json!({ "v": 1, "at": SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64 }))?;
    println!("{}", json!({ "name": name, "host": wired.host }));
    Ok(())
}

fn dev_space_down() -> Res<()> {
    let home = dev_home();
    let cfg = shroom_e2e_live::credentials()?;
    // The run is the one the dev space was set up under (its label ends in the run id); with
    // none, crashed runs are still reaped.
    let label = std::fs::read(home.join("space.json")).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()).and_then(|v| v["label"].as_str().map(String::from));
    let run = label.as_deref().and_then(shroom_cloud::wire::run_of);
    let gone = shroom_e2e_live::close(&cfg, run)?;
    for f in ["space.json", "primed.json"] {
        let _ = std::fs::remove_file(home.join(f));
    }
    if harness_token(&home) == Some(true) {
        let _ = std::fs::remove_file(home.join("token.json"));
    }
    println!("{}", json!({ "removed": gone }));
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("dev-space") if args.get(2).map(String::as_str) == Some("up") && args.len() == 5 => dev_space_up(Path::new(&args[3]), Path::new(&args[4])),
        Some("dev-space") if args.get(2).map(String::as_str) == Some("down") => dev_space_down(),
        _ => Err("usage: shroom-dev dev-space up <worker.js> <hls.min.js> | dev-space down".into()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

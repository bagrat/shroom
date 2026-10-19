mod commands;
#[cfg(feature = "dev-harness")]
mod drives;
#[cfg(feature = "dev-harness")]
mod front;
#[cfg(feature = "dev-harness")]
mod harness;
mod runtime;
#[cfg(feature = "dev-harness")]
mod take_drive;
#[cfg(feature = "dev-harness")]
mod tray_probe;

/// Marks a binary built with the test harness, so `tools/package-release.sh` refuses to
/// package it as a release (DECISIONS B62). `main` reads it through `black_box`, so neither
/// the optimizer nor the linker's dead-strip can drop it; the words appear in no build
/// without the feature.
#[cfg(feature = "dev-harness")]
static SHROOM_HARNESS_BUILD: [u8; 40] = *b"shroom:dev-harness-build:never-a-release";

/// Links opened before the runtime exists (a cold launch by the page's Open shroom or Edit
/// in shroom): held in memory, handed over once it's up.
static EARLY_LINKS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Hands every early link to `runtime::opened`, behind `Booted` on the main thread.
fn drain_early_links(app: &tauri::AppHandle, rt: &runtime::Runtime) {
    let links = std::mem::take(&mut *EARLY_LINKS.lock().unwrap());
    if links.is_empty() {
        return;
    }
    let rt = rt.clone();
    let _ = app.run_on_main_thread(move || {
        for link in links {
            runtime::opened(&rt, &link);
        }
    });
}

fn main() {
    #[cfg(feature = "dev-harness")]
    std::hint::black_box(&SHROOM_HARNESS_BUILD);
    let args: Vec<String> = std::env::args().collect();
    let context = tauri::generate_context!();
    // The test harness's modes (--selftest, --drive, …) run only in a dev identifier's
    // build, and only one built with the harness — refused before anything else, the early
    // exits (--camera-check, --selftest) included (B5, whole-spec audit b4 #18).
    let dev = runtime::Flavor::of(&context.config().identifier).dev();
    // A release runs only its bundled encoder; a dev build may take another (B82).
    runtime::set_encoder_road(dev);
    if let Some(why) = runtime::harness_refusal(&args, dev, cfg!(feature = "dev-harness")) {
        eprintln!("shroom: {why}");
        std::process::exit(64);
    }
    // A dev build reopened by its own update carries its throwaway home in a note beside
    // the throwaway copy (never on the reopen's command line, which only says how it went).
    #[cfg(feature = "dev-harness")]
    let args = if dev { runtime::dev_relaunch_args(args) } else { args };
    let flags = runtime::DevFlags::from_args(&args);
    let launch_args = args.clone();
    #[cfg(feature = "dev-harness")]
    let mode = harness::from_args(args.clone());
    #[cfg(feature = "dev-harness")]
    if let harness::Mode::CameraCheck { out, record } = &mode {
        std::process::exit(harness::camera_check(out, *record));
    }
    #[cfg(feature = "dev-harness")]
    if let harness::Mode::PlayOut { hz, at_ms, seconds } = &mode {
        std::process::exit(harness::play_out(*hz, *at_ms, *seconds));
    }
    #[cfg(feature = "dev-harness")]
    if let harness::Mode::Selftest(out) = &mode {
        std::process::exit(harness::selftest(out));
    }

    let builder = tauri::Builder::default().plugin(tauri_nspanel::init());
    #[cfg(feature = "webdriver")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());

    let mut app = builder
        // The Library's posters: only the files `posters::serve` allows, by checked names.
        .register_uri_scheme_protocol(runtime::posters::SCHEME, |ctx, request| {
            use tauri::Manager;
            let body = ctx.app_handle().try_state::<runtime::Runtime>().and_then(|rt| runtime::posters::serve(rt.inner(), request.uri().path()));
            let builder = tauri::http::Response::builder().header("Cache-Control", "no-store");
            match body {
                Some(b) => builder.status(200).header("Content-Type", "image/jpeg").body(b),
                None => builder.status(404).body(Vec::new()),
            }
            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
        })
        // The editor's video and waveform (N63): only what its lane registered, by checked
        // names, to the editor's webview alone — a range read off the main thread.
        .register_asynchronous_uri_scheme_protocol(runtime::media::SCHEME, |ctx, request, responder| {
            let editor = ctx.webview_label() == runtime::presenter::EDITOR;
            std::thread::spawn(move || {
                let range = request.headers().get("range").and_then(|v| v.to_str().ok());
                let a = if editor { runtime::media::answer(&runtime::media::MEDIA, request.method().as_str(), request.uri().path(), range) } else { runtime::media::Answer { status: 404, headers: Vec::new(), body: Vec::new() } };
                let mut b = tauri::http::Response::builder().status(a.status);
                for (k, v) in &a.headers {
                    b = b.header(*k, v);
                }
                responder.respond(b.body(a.body).unwrap_or_else(|_| tauri::http::Response::new(Vec::new())));
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::act,
            commands::key,
            commands::dismiss,
            commands::popover_keyed,
            commands::scene_get,
            commands::popover_fit,
            commands::paste_secret,
            commands::field,
            commands::webview_error,
            commands::edit
        ])
        .setup(move |app| {
            use tauri::Manager;
            // Setup runs inside `applicationDidFinishLaunching:`, while the launch event is
            // the one being handled: a login launch is read from it (N41).
            let launch = runtime::launch_kind(&launch_args, runtime::launch::launched_at_login());
            // ⌘Q and the Dock's Quit ask the core (law 4).
            runtime::launch::hook_terminate(app.handle());
            // The dev-only flags never reach a release build's runtime, and a dev build
            // takes the fake only toward this Mac with a throwaway home (every mode).
            if flags.any() && !dev {
                eprintln!("shroom: --home, --cloud-fake, --live-shell, --releases, --update-target, --as-version, --space-base and --e2e are for dev builds only");
                std::process::exit(64);
            }
            if let Some(why) = flags.refusal() {
                eprintln!("shroom: {why}");
                std::process::exit(64);
            }
            // A dev build updates only a throwaway copy from a throwaway home: its reopen
            // without that home is never run (it would land on the real dev home).
            if dev && matches!(launch, shroom_core::world::LaunchKind::UpdateRelaunch | shroom_core::world::LaunchKind::UpdateFailed) && flags.home.is_none() {
                eprintln!("shroom: a dev build's update relaunch runs only with its throwaway --home");
                std::process::exit(64);
            }
            #[cfg(feature = "dev-harness")]
            match mode {
                harness::Mode::Grant => {
                    std::thread::spawn(harness::grant);
                    return Ok(());
                }
                harness::Mode::RecordCheck { out, seconds, capture_fps, audio, exclude_pattern } => {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::record_check(handle, out, seconds, capture_fps, audio, exclude_pattern));
                    return Ok(());
                }
                harness::Mode::ShellProbe(out) => {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::shell_probe(handle, out));
                    return Ok(());
                }
                harness::Mode::GuardProbe(out) => {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::guard_probe(handle, out));
                    return Ok(());
                }
                harness::Mode::Drive { out, seconds } => {
                    let home = match &flags.home {
                        Some(h) => runtime::home::Home::at(h.clone()),
                        None => runtime::home::Home::for_identifier(&app.config().identifier).unwrap_or_else(|e| {
                            eprintln!("shroom: {e}");
                            std::process::exit(70);
                        }),
                    };
                    if let Err(e) = harness::prepare_drive_home(&home) {
                        eprintln!("shroom: the drive's home: {e}");
                        std::process::exit(70);
                    }
                    let rt = runtime::start_with(app.handle(), shroom_core::world::LaunchKind::LoginItem, true, &flags);
                    app.manage(rt.clone());
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::drive(handle, rt, out, seconds));
                    return Ok(());
                }
                harness::Mode::DriveOnboarding { out, taken, gate_marker, resume, doors, invite, token_file } => {
                    // Against the fake from a throwaway home — or, the e2e run, against the
                    // test account (`--e2e`) with the run's own token, handed over by file.
                    let live = flags.e2e && token_file.is_some();
                    if !live && (flags.home.is_none() || flags.cloud_fake.is_none()) {
                        eprintln!("shroom: --drive onboarding needs --home <dir> and --cloud-fake <api> <r2> (or --e2e and --token-file <file>)");
                        std::process::exit(64);
                    }
                    if token_file.is_some() && !flags.e2e {
                        eprintln!("shroom: --drive onboarding --token-file is the e2e run's: pass --e2e");
                        std::process::exit(64);
                    }
                    // Resumed (L3, B69): the person's own launch of a home whose first one
                    // was killed mid-wire — no install flag.
                    if resume && !matches!(launch, shroom_core::world::LaunchKind::User) {
                        eprintln!("shroom: --drive onboarding --resume is the person's own launch: no --relaunched flag");
                        std::process::exit(64);
                    }
                    // A fresh install's first launch, as the install script opens it
                    // (`--relaunched=install`), against a throwaway home and the fake.
                    if !resume && !matches!(launch, shroom_core::world::LaunchKind::Install { join: false }) {
                        eprintln!("shroom: --drive onboarding is the installer's launch: pass --relaunched=install");
                        std::process::exit(64);
                    }
                    let rt = runtime::start_traced(app.handle(), launch, &flags, runtime::Trace::onboarding);
                    app.manage(rt.clone());
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::drive_onboarding(handle, rt, out, taken, gate_marker, resume, (doors, invite), token_file));
                    return Ok(());
                }
                harness::Mode::DriveRegion5 { which, out } => {
                    if flags.home.is_none() || flags.cloud_fake.is_none() {
                        eprintln!("shroom: --drive {} needs --home <dir> and --cloud-fake <api> <r2>", which.name());
                        std::process::exit(64);
                    }
                    if which == harness::Region5::Update && (flags.releases.is_none() || flags.update_target.is_none()) {
                        eprintln!("shroom: --drive update needs --releases <loopback> and --update-target <throwaway .app>");
                        std::process::exit(64);
                    }
                    // A provisioned Mac, against a throwaway home and the fake (the drive
                    // seeds both before the core boots).
                    if let Err(e) = harness::seed_provisioned(&flags) {
                        eprintln!("shroom: seeding the drive's home: {e}");
                        std::process::exit(70);
                    }
                    let kind = if which == harness::Region5::UpdateRelaunched { launch } else { shroom_core::world::LaunchKind::LoginItem };
                    let rt = runtime::start_traced(app.handle(), kind, &flags, which.probe());
                    app.manage(rt.clone());
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::drive_region5(handle, rt, which, out));
                    return Ok(());
                }
                harness::Mode::DriveRegion6 { which, out } => {
                    // The heartbeat's member is live: a throwaway home and the fenced test
                    // space its invite names — never the fake or a local Worker (B68).
                    let live = which == harness::Region6::HbMember;
                    if live && (flags.home.is_none() || flags.cloud_fake.is_some() || flags.space_base.is_some()) {
                        eprintln!("shroom: --drive hb-member needs --home <dir> and no --cloud-fake or --space-base (it joins the heartbeat's live test space)");
                        std::process::exit(64);
                    }
                    if !live && (flags.home.is_none() || flags.cloud_fake.is_none() || flags.space_base.is_none()) {
                        eprintln!("shroom: --drive {} needs --home <dir>, --cloud-fake <api> <r2> and --space-base <the local Worker>", which.name());
                        std::process::exit(64);
                    }
                    // The throwaway home and the local Worker, seeded before the core boots.
                    let seed = match harness::seed_region6(which, &flags) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("shroom: seeding the drive: {e}");
                            std::process::exit(70);
                        }
                    };
                    // The join drive is the installer's launch (`--invite`); the others a login.
                    let kind = if matches!(which, harness::Region6::Join | harness::Region6::L6Member | harness::Region6::HbMember) { shroom_core::world::LaunchKind::Install { join: true } } else { shroom_core::world::LaunchKind::LoginItem };
                    let rt = runtime::start_traced(app.handle(), kind, &flags, which.probe());
                    app.manage(rt.clone());
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::drive_region6(handle, rt, which, out, seed));
                    return Ok(());
                }
                harness::Mode::DriveLaunch { out, provisioned, late } => {
                    if flags.home.is_none() || (provisioned && flags.cloud_fake.is_none()) {
                        eprintln!("shroom: --drive launch needs --home <dir> (and, provisioned, --cloud-fake <api> <r2>)");
                        std::process::exit(64);
                    }
                    // A set-up Mac (the fake's `found` space): a launch the person did
                    // opens config, the launch-as-click (N41).
                    if provisioned && let Err(e) = harness::seed_provisioned(&flags) {
                        eprintln!("shroom: seeding the drive's home: {e}");
                        std::process::exit(70);
                    }
                    // However this process was started (the argv, the launch event): what
                    // the launch opens by itself.
                    let rt = runtime::start_traced(app.handle(), launch, &flags, runtime::Trace::onboarding);
                    app.manage(rt.clone());
                    // A start-up that keeps this thread busy after the mushroom is made, for
                    // longer than a popover waits for it to settle: the event loop's first
                    // turn comes late, with the boot's Scene already waiting. (Read on a
                    // clone without this: the first turn 1.1 s after the mushroom, in runs
                    // of launches at a time, against 0.3 s in the others.)
                    if late {
                        std::thread::sleep(std::time::Duration::from_millis(1200));
                    }
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::drive_launch(handle, rt, launch, out));
                    return Ok(());
                }
                harness::Mode::DriveTake { which, out } => {
                    // The show pass (B127) records on the e2e run's own space, from the home
                    // its wizard set up; every other take drive runs on the fake.
                    let show = which == take_drive::TakeDrive::Show;
                    if show && !flags.e2e {
                        eprintln!("shroom: --drive take show is the e2e run's: pass --e2e");
                        std::process::exit(64);
                    }
                    if !show && (flags.home.is_none() || flags.cloud_fake.is_none()) {
                        eprintln!("shroom: --drive take needs --home <dir> and --cloud-fake <api> <r2>");
                        std::process::exit(64);
                    }
                    // A provisioned Mac recording, against a throwaway home and the fake.
                    if !show && let Err(e) = harness::seed_provisioned(&flags) {
                        eprintln!("shroom: seeding the drive's home: {e}");
                        std::process::exit(70);
                    }
                    let rt = runtime::start_traced(app.handle(), shroom_core::world::LaunchKind::LoginItem, &flags, which.probe());
                    app.manage(rt.clone());
                    let handle = app.handle().clone();
                    std::thread::spawn(move || take_drive::drive(handle, rt, which, out));
                    return Ok(());
                }
                harness::Mode::TrayProbe(out) => {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || tray_probe::tray_probe(handle, out));
                    return Ok(());
                }
                harness::Mode::BubbleProbe { out, hold, window } => {
                    let handle = app.handle().clone();
                    std::thread::spawn(move || harness::bubble_probe(handle, out, hold, window));
                    return Ok(());
                }
                _ => {}
            }
            let rt = runtime::start(app.handle(), launch, &flags);
            app.manage(rt.clone());
            drain_early_links(app.handle(), &rt);
            Ok(())
        })
        .build(context)
        .expect("shroom failed to start");
    // A menu-bar app from the first moment: the bundle is born one (LSUIElement), and the
    // windowing layer, which applies its launch policy as the app finishes launching —
    // an ordinary app's, unless told — is told here, before its loop runs. Set only once
    // launched, shroom was an ordinary app for a moment at every start, and the Dock was
    // told so. The Dock icon comes with a window, raised when a Scene asks (law 6,
    // DECISIONS B113).
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    app.run(|app, event| {
        use tauri::Manager;
        match event {
            tauri::RunEvent::Reopen { .. } => {
                // A launch you did while running is a click (N41).
                if let Some(rt) = app.try_state::<runtime::Runtime>() {
                    let rt = rt.inner().clone();
                    if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
                        // The installer's `open … --args --invite` on a running shroom
                        // reopens it: an invite it just wrote is an arrival (§6.12).
                        runtime::member::check_invite_file(&rt, true);
                        let (quick, _) = runtime::probe::quick(mtm, &rt.home.root);
                        rt.post(shroom_core::event::Event::Reopened(quick));
                        runtime::presenter::bring_forward(app);
                    }
                }
            }
            // The page's Open shroom (`shroom://join/<host>/<code>`), or a join link handed
            // over by `open -a shroom <link>` (N50): an arrival, queued behind `Booted` on
            // the main thread; the owner page's Edit in shroom (`shroom://edit/…`, N69) goes
            // to the core the same way. Any other web address is ignored. The link's text goes to
            // the stash and `~/.shroom/invite` only — never a log. The `shroom` URL type
            // belongs in the release bundle's Info.plist only: a dev bundle never claims
            // the scheme, so a real invite opened on this Mac can't launch a dev build.
            tauri::RunEvent::Opened { urls } => {
                EARLY_LINKS.lock().unwrap().extend(urls.iter().map(|u| u.to_string()).filter(|u| shroom_cloud::member::takes_opened(u)));
                if let Some(rt) = app.try_state::<runtime::Runtime>() {
                    drain_early_links(app, rt.inner());
                }
            }
            // A window going away never ends a menu-bar app; only an exit shroom asked
            // for does (`code` is set by `app.exit`).
            tauri::RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            tauri::RunEvent::Exit => {
                if let Some(rt) = app.try_state::<runtime::Runtime>() {
                    runtime::exiting(rt.inner());
                }
            }
            _ => {}
        }
    });
}

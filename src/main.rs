use ghost::cli::{Cli, Commands};
use ghost::config::GhostConfig;
use ghost::gadgets::default_gadgets;
use ghost::interceptor::{CommandWrapper, ProxyStub};
use ghost::session::Session;
use ghost::tui::TuiRenderer;
use std::io::IsTerminal;

/// ghost 👻 binary entry.
/// Thin: clap dispatch + voice-flavored startup messages + full wiring.
/// Real work lives in the layers (see lib.rs + modules).
///
/// full v1: attach/proxy/run/replay/gadgets/config subcommands, --headless + --config (toml/serde),
/// basic recording + replay, headless non-ratatui voice path, safety dry banners everywhere.
///
/// All public text (help, banners, examples in comments) MUST match @ThatbV voice:
/// spooky, kaomoji >:[ (¬‿¬) (｡◕‿↼) 💀 XX lmao , blunt roasts, "zero chill",
/// "they ALL talk eventually", direct security research, anti-corporate.
/// No corporate voice. Ever.
fn main() {
    let cli = Cli::parse_cli();

    // headless: --headless global (from clap) or auto when no tty (pipes, ci, scripts). non-ratatui voice path.
    let is_headless = cli.headless || !std::io::stdout().is_terminal();

    // config load if --config provided. seeds gadgets, can override dry (cli wins), voice prefs available.
    let _loaded_cfg: Option<GhostConfig> =
        cli.config.as_ref().and_then(|p| GhostConfig::load(p).ok());

    match cli.command {
        Commands::Attach {
            command,
            gadgets,
            dry_run,
        } => {
            let target = command.join(" ");
            println!("👻 ghost attaching to: {}", target);
            println!("gadgets armed: {:?}", gadgets);
            println!(
                "dry_run: {}  (real mutations only on explicit opt-in. safety first lmao)",
                dry_run
            );

            let mut session = Session::new(&target);
            session.dry_run = dry_run;
            if !dry_run {
                // in real: would warn hard + require confirm. fail loud.
                println!(
                    ">:[ WARNING: dry_run=false. mutations will be real. they ALL talk eventually XX"
                );
            }

            // full config wiring: --config or --gadgets seeds the armed list (select uses existing gadgets)
            let mut g_list = gadgets;
            if let Some(ref cfg) = _loaded_cfg
                && !cfg.gadgets.is_empty()
            {
                g_list = cfg.gadgets.clone();
            }
            session.select_gadgets(&g_list);

            // list armed (voice descs from gadget trait / registry)
            for g in &session.active_gadgets {
                println!("  - {} : {}", g.name(), g.description());
            }

            // real interceptor: command wrapper (TDD impl from priors). emits banner + CommandOutput.
            // feeds the event bus via session.attach_with_interceptor (ingest + gadgets + personality + state + lines)
            // dry_run passed down fully. banners always "👻 ghost attached ..."
            println!("wrapping with command interceptor (👻 attached banner + capture)...");
            let wrapper = CommandWrapper::new(command);
            let events = wrapper.run(dry_run);
            session.attach_with_interceptor(events);

            // basic recording save (for replay cmd). uses personality_lines + voice banners collected in session.
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let rec_id = format!("attach-{}", ts);
            if let Ok(p) = session.save_recording(&rec_id) {
                // advertise the id we actually saved under (was printing the bare
                // ts, which pointed replay at a file that does not exist).
                println!("recording saved: {} (use: ghost replay {})", p, rec_id);
            }

            if is_headless {
                let renderer = TuiRenderer::new();
                renderer.run_headless(&session);
            } else {
                // show captured for tui case (voice roasts from personality via bus)
                println!(
                    "--- event stream ({} total, bus ingested) ---",
                    session.events.len()
                );
                for ev in &session.events {
                    match ev {
                        ghost::Event::CommandOutput { line, stream, .. } => {
                            println!("  [{}] {}", stream, line);
                        }
                        ghost::Event::LogLine { msg, source, .. } => {
                            if source.starts_with("gadget:")
                                || msg.contains("👻")
                                || msg.contains("ghost attached")
                            {
                                println!("  {}", msg);
                            } else {
                                println!("  [log:{}] {}", source, msg);
                            }
                        }
                        _ => println!("  {:?}", ev),
                    }
                }
                println!("{}", session.summary());
                println!("(they ALL talk eventually XX)");

                // real TUI (ratatui face + activity glitch + gadget bar + status + log overlays, consumes for keys)
                let renderer = TuiRenderer::new();
                renderer.run(session);
            }
        }

        Commands::Proxy { addr } => {
            println!("👻 ghost proxy mode on {}", addr);
            println!("tokio backend stub. simple http/raw for v1. (¬‿¬)");
            println!("> this is where the live stream + gadget slots will live.");

            let mut session = Session::new(format!("proxy:{}", addr));
            // proxy stub respects dry_run in banner text
            println!("attaching proxy stub interceptor...");
            let proxy = ProxyStub::new(addr);
            let events = proxy.run(session.dry_run);
            session.attach_with_interceptor(events);

            println!("--- proxy event stream ---");
            for ev in &session.events {
                if let ghost::Event::LogLine { msg, .. } = ev {
                    println!("  {}", msg);
                }
            }
            println!("{}", session.summary());

            if is_headless {
                let renderer = TuiRenderer::new();
                renderer.run_headless(&session);
            } else {
                let renderer = TuiRenderer::new();
                renderer.run(session);
            }
        }

        Commands::Run { config } => {
            println!(
                "👻 loading config from {} ... (¬‿¬) zero chill. batch run.",
                config
            );
            let cfg = GhostConfig::load(&config).unwrap_or_else(|_| GhostConfig::with_defaults());
            println!(
                "config: gadgets={:?} dry={} kaomoji_level={}",
                cfg.gadgets, cfg.dry_run, cfg.voice.kaomoji_level
            );

            // batch over targets from config (or default). for v1 simple: one session per, or combined.
            // use first or all; wire real attach would loop, here simulate + record one combined for replay.
            let mut session = Session::new("config-run");
            session.dry_run = cfg.dry_run;

            // synthetic to exercise + voice
            session.activate_gadget(if cfg.gadgets.iter().any(|g| g == "roast") {
                "roast"
            } else {
                "poke"
            });

            if is_headless {
                let renderer = TuiRenderer::new();
                renderer.run_headless(&session);
            } else {
                println!("{}", session.summary());
                let renderer = TuiRenderer::new();
                renderer.run(session);
            }
        }

        Commands::Replay { session_id } => {
            println!(
                "👻 replaying session {} (¬‿¬) they ALL talk eventually XX",
                session_id
            );
            let _ = TuiRenderer::replay(&session_id);
            println!("replay done. zero chill 💀");
        }

        Commands::Hook { sentinel, mode } => {
            // the PreToolUse bridge. read the tool call, run offense, defer to
            // sentinel, narrate the verdict. claude code reads our STDOUT as the
            // decision; the voice goes to stderr + the watch log (never stdout).
            use ghost::bridge::{BridgeConfig, BridgeMode, SubprocessSentinel, run_bridge};
            use std::io::Read;

            let mut input = String::new();
            let _ = std::io::stdin().read_to_string(&mut input);

            let sentinel_cmd = sentinel.unwrap_or_else(|| "sentinel".to_string());
            let oracle = SubprocessSentinel::new(sentinel_cmd, vec!["evaluate".to_string()]);

            let mut cfg = BridgeConfig::default();
            if let Some(m) = mode.as_deref() {
                cfg.mode = match m {
                    "shadow-attack" => BridgeMode::ShadowAttack,
                    "live-attack" => BridgeMode::LiveAttack,
                    _ => BridgeMode::Observe,
                };
            }

            let engine = ghost::PersonalityEngine::new();
            let outcome = run_bridge(&input, &engine, &oracle, &cfg);

            // structured feed: EVERY call (pass or block) lands in ~/.ghost/events.jsonl
            // so `ghost watch` can drive the face live and `ghost blocks` can tell
            // you what the agent keeps trying. best-effort; never gates the decision.
            let record = ghost::watchlog::CallRecord::from_outcome(&outcome, now_ms());
            ghost::watchlog::append_call(&record);

            if let Some(ev) = &outcome.block_event {
                // narrate to you: the watch log + stderr (claude surfaces hook stderr)
                let line = format!("👻 {} {}", outcome.face.emoji(), ev);
                append_block_log(&line);
                eprintln!("{line}");
            }

            // the decision claude code actually enforces. stdout = JSON only.
            println!("{}", outcome.hook_stdout);
        }

        Commands::Install {
            sentinel,
            uninstall,
        } => {
            use ghost::bridge::{install_into_settings, uninstall_from_settings};

            let settings_path = claude_settings_path();
            let current = std::fs::read_to_string(&settings_path).unwrap_or_default();

            if uninstall {
                match uninstall_from_settings(&current) {
                    Ok(updated) => {
                        let _ = write_settings(&settings_path, &updated);
                        println!(
                            "👻 ghost bridge yanked from {}. back to whatever defense you had, lone wolf >:[ XX",
                            settings_path
                        );
                    }
                    Err(e) => eprintln!("couldn't uninstall, settings.json is cursed: {e} >:[ 💀"),
                }
            } else {
                let ghost_bin = std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "ghost".to_string());
                let sentinel_cmd = sentinel
                    .or_else(which_sentinel)
                    .unwrap_or_else(|| "sentinel".to_string());

                match install_into_settings(&current, &ghost_bin, &sentinel_cmd) {
                    Ok(updated) => match write_settings(&settings_path, &updated) {
                        Ok(()) => {
                            println!("👻 ghost bridge installed into {} (¬‿¬)", settings_path);
                            println!(
                                "  ghost now wraps sentinel ({}) on every tool call.",
                                sentinel_cmd
                            );
                            println!(
                                "  offense runs, sentinel rules, ghost roasts the blocks in your voice."
                            );
                            println!(
                                "  blocks land in ~/.ghost/blocks.log. they ALL talk eventually XX 💀"
                            );
                        }
                        Err(e) => eprintln!(">:[ wrote nothing, fix your perms: {e} 💀"),
                    },
                    Err(e) => {
                        eprintln!(">:[ install failed: {e}. is your settings.json valid json? 💀")
                    }
                }
            }
        }

        Commands::Watch { path } => {
            // the live view: tail the bridge feed and react. point it at the
            // structured log `ghost hook` writes, default ~/.ghost/events.jsonl.
            let feed = path
                .map(std::path::PathBuf::from)
                .or_else(ghost::watchlog::events_log_path)
                .unwrap_or_else(|| std::path::PathBuf::from(".ghost/events.jsonl"));
            println!(
                "👻 ghost watch -> {} (¬‿¬) tailing the bridge. (run `ghost install` first if it's empty). q to quit XX",
                feed.display()
            );
            let renderer = TuiRenderer::new();
            if is_headless {
                renderer.run_watch_headless(feed);
            } else {
                renderer.run_watch(feed);
            }
        }

        Commands::Gadgets => {
            println!("👻 available gadgets (v1). slot these. hotkeys coming.");
            println!("------------------------------------------------");
            for g in default_gadgets() {
                println!("{}  -- {}", g.name(), g.description());
            }
            println!("------------------------------------------------");
            println!("use with --gadgets poke,roast on attach. or via --config. more in spec.");
            println!("(｡◕‿↼) they ALL talk eventually XX");
        }

        Commands::Config { show, path } => {
            let p = path
                .or(cli.config.clone())
                .unwrap_or_else(|| "ghost.toml".to_string());
            let cfg = if show {
                GhostConfig::load(&p).unwrap_or_else(|_| GhostConfig::with_defaults())
            } else {
                GhostConfig::with_defaults()
            };
            println!("👻 ghost config (toml) at {} (¬‿¬)", p);
            println!("  gadgets: {:?}", cfg.gadgets);
            println!(
                "  dry_run default: {} (safety. override with --dry-run=false)",
                cfg.dry_run
            );
            println!("  voice: kaomoji_level={}", cfg.voice.kaomoji_level);
            println!("  targets (for run): {:?}", cfg.targets);
            println!(
                "use --config {} on attach/run to load. they ALL talk eventually XX",
                p
            );
        }
    }
}

/// wall-clock milliseconds since the unix epoch, for the structured feed.
/// (the live event model uses monotonic Instant; the cross-process feed needs
/// a real clock that survives the hook subprocess boundary.)
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// append a block narration line to the watch log (~/.ghost/blocks.log).
/// the side channel a live `ghost watch` would tail. best-effort, never panics.
fn append_block_log(line: &str) {
    use std::io::Write;
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let dir = std::path::Path::new(&home).join(".ghost");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("blocks.log"))
    {
        let _ = writeln!(f, "{line}");
    }
}

/// ~/.claude/settings.json (where claude code reads PreToolUse hooks).
fn claude_settings_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home}/.claude/settings.json")
}

/// write settings.json, creating parent dirs. pretty json (it's user-editable).
fn write_settings(path: &str, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

/// best-effort `which sentinel` so install can self-configure.
fn which_sentinel() -> Option<String> {
    let out = std::process::Command::new("which")
        .arg("sentinel")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

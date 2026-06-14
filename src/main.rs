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
                println!("recording saved: {} (use: ghost replay {})", p, ts);
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

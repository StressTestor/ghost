use ghost::cli::{Cli, Commands};
use ghost::gadgets::default_gadgets;
use ghost::interceptor::{CommandWrapper, ProxyStub};
use ghost::session::Session;
use ghost::tui::TuiRenderer;

/// ghost 👻 binary entry.
/// Thin: clap dispatch + voice-flavored startup messages.
/// Real work lives in the layers (see lib.rs + modules).
///
/// All public text (help, banners, examples in comments) MUST match @ThatbV voice:
/// spooky, kaomoji >:[ (¬‿¬) (｡◕‿↼) 💀 XX lmao , blunt roasts, "zero chill",
/// "they ALL talk eventually", direct security research, anti-corporate.
/// No corporate voice. Ever.
fn main() {
    let cli = Cli::parse_cli();

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
                // in real: would warn hard + require confirm
                println!(
                    ">[: WARNING: dry_run=false. mutations will be real. they ALL talk eventually XX"
                );
            }

            // list armed (note: --gadgets filter not applied in v1 session load, uses defaults)
            for g in default_gadgets() {
                println!("  - {} : {}", g.name(), g.description());
            }

            // real interceptor: command wrapper (TDD impl). emits banner + CommandOutput.
            // feeds the event bus via session.attach_with_interceptor (ingest + gadgets + personality + state)
            println!("wrapping with command interceptor (👻 attached banner + capture)...");
            let wrapper = CommandWrapper::new(command);
            let events = wrapper.run(dry_run);
            session.attach_with_interceptor(events);

            // show captured (headless print for now; tui stub after for continuity)
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

            // tui stub kept for now (prints extra summary + face tease). real TUI in parallel work.
            let renderer = TuiRenderer::new();
            renderer.run(&session);
        }

        Commands::Proxy { addr } => {
            println!("👻 ghost proxy mode on {}", addr);
            println!("tokio backend stub. simple http/raw for v1. (¬‿¬)");
            println!("> this is where the live stream + gadget slots will live.");

            let mut session = Session::new(format!("proxy:{}", addr));
            // real basic proxy stub (emits banner + simulated events, no real listener yet)
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

            let renderer = TuiRenderer::new();
            renderer.run(&session);
        }

        Commands::Run { config } => {
            println!("👻 loading config from {} ... (stub)", config);
            println!(
                "zero chill. running session from toml. would apply gadgets + start interceptor or tui."
            );
            let session = Session::new("config-run");
            println!("{}", session.summary());
        }

        Commands::Replay { session_id } => {
            println!("👻 replaying session {} (stub)", session_id);
            println!("would render events + personality lines + ghost face states from recording.");
            println!("for science. lmao");
        }

        Commands::Gadgets => {
            println!("👻 available gadgets (v1). slot these. hotkeys coming.");
            println!("------------------------------------------------");
            for g in default_gadgets() {
                println!("{}  -- {}", g.name(), g.description());
            }
            println!("------------------------------------------------");
            println!("use with --gadgets poke,roast on attach. more in spec.");
            println!("(｡◕‿↼) they ALL talk eventually XX");
        }
    }
}

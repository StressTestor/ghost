use ghost::cli::{Cli, Commands};
use ghost::gadgets::default_gadgets;
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

            let session = Session::new(&target);
            if !dry_run {
                // in real: would warn hard + require confirm
                println!(
                    ">[: WARNING: dry_run=false. mutations will be real. they ALL talk eventually XX"
                );
            }
            // demo: fake some events through to show the bus + personality
            // (real interceptor will feed this)
            for g in default_gadgets() {
                println!("  - {} : {}", g.name(), g.description());
            }

            // stub ingest to exercise
            // (in follow on: actual wrapper feeds live)
            println!("(stub) running fake ingest for skeleton demo...");
            // can't easily make real Event here without pub use, but lib reexports
            // for demo just print and use tui
            let renderer = TuiRenderer::new();
            renderer.run(&session);
        }

        Commands::Proxy { addr } => {
            println!("👻 ghost proxy mode on {}", addr);
            println!("tokio backend stub. simple http/raw for v1. (¬‿¬)");
            println!("> this is where the live stream + gadget slots will live.");
            let session = Session::new(format!("proxy:{}", addr));
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

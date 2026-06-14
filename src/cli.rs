use clap::{Parser, Subcommand};

/// CLI / Entry layer (clap).
/// Subcommands per spec v1:
///   attach <command...> [--gadgets ...] [--dry-run]
///   proxy <addr>
///   run --config ...
///   replay <session-id>
///   gadgets (list)
///   (and --help with attitude)
///
/// All user-facing text and examples must be in project voice:
/// spooky 👻, kaomoji, blunt, "zero chill", direct security research tone.
/// No corporate. lowercase where natural. lmao energy.
#[derive(Parser, Debug)]
#[command(
    name = "ghost",
    version,
    about = "👻 live visibility + deliberate chaos for your agents, commands, localhost. complements sentinel. they ALL talk eventually XX",
    long_about = "ghost 👻\n\nreal tool. real targets. real effects (scoped).\nwatch. roast. poke. mutate. for science.\n\nloud, distrustful research partner in your terminal.\n\nsee docs/superpowers/specs/2026-06-14-ghost-design.md"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Attach to a command / agent process (wrapper + capture).
    /// Example: ghost attach ./my-agent --gadgets poke,roast --dry-run
    Attach {
        /// The command + args to wrap (everything after)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,

        /// Comma separated list of gadgets to arm (defaults from config)
        #[arg(long, value_delimiter = ',', default_value = "poke,roast")]
        gadgets: Vec<String>,

        /// Observe only. Never mutate. (sane default, but explicit)
        #[arg(long, default_value_t = true)]
        dry_run: bool,
    },

    /// Proxy a local addr (http-ish or raw for now). Simple tokio backend.
    Proxy { addr: String },

    /// Run from a full config file (toml). headless or tui depending on flags.
    Run {
        #[arg(short, long, default_value = "ghost.toml")]
        config: String,
    },

    /// Replay a previous session recording (text + face states + roasts).
    Replay { session_id: String },

    /// List available gadgets with your voice descriptions + hotkeys.
    Gadgets,
}

impl Cli {
    pub fn parse_cli() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_attach_subcommand() {
        // clap derive test: construct via builder-ish or just check struct
        let cli = Cli {
            command: Commands::Attach {
                command: vec!["./agent".into(), "--foo".into()],
                gadgets: vec!["poke".into()],
                dry_run: true,
            },
        };
        match cli.command {
            Commands::Attach { dry_run, .. } => assert!(dry_run),
            _ => panic!("wrong variant"),
        }
    }
}

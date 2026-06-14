use crate::session::Session;

/// Renderer (TUI) layer using ratatui + crossterm.
///
/// v1 layout (dense but readable, per spec):
/// - Top: ghost face (big, emoji + kaomoji blocks) + title "ghost 👻" + current target
/// - Main: activity canvas (glitchy event stream + effects)
/// - Right: gadget bar (slots, hotkeys, voice descs)
/// - Bottom: status strip ("ZERO CHILL", "THEY TALKING YET?", "CHAOS FOR SCIENCE") + mini live log
/// - Overlays for help (your voice), confirm mutation
///
/// Effects: color flashes on roasts, face state machine (neutral 👻, (¬‿¬), >:[ , party)
///
/// Boundaries:
/// - Renderer ONLY consumes events + personality + state; NO business logic, NO mutation.
/// - Keyboard first. Mouse optional.
/// - Resize aware.
///
/// This is pure stub for skeleton. Real ratatui widgets, event loop, face drawing in next steps.
/// Headless mode bypasses this entirely (still gets personality output).
pub struct TuiRenderer {
    // terminal state etc later
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// Run the full TUI loop. Blocks until exit.
    /// In skeleton: just prints a spooky banner + session summary then "would enter tui".
    /// (prevents accidental real TUI until impl)
    pub fn run(&self, session: &Session) {
        println!("👻 ghost tui engaged (stub)");
        println!("target: {}", session.target);
        println!("{}", session.summary());
        println!("> press q to quit in real version. zero chill mode on.");
        println!("(¬‿¬) they ALL talk eventually XX");
        // real: crossterm enable raw, ratatui Terminal::new, draw loop, input poll
    }

    /// Headless / reporter path (no TUI). Still runs personality for artifacts.
    pub fn headless_summary(&self, session: &Session) -> String {
        format!(
            "ghost 👻 headless run complete\n{}\nroasts fired: {}\n-- end trace --",
            session.summary(),
            session.roast_count
        )
    }
}

impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

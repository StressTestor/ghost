//! ghost 👻
//! Core library for the ghost binary: interception, gadgets, personality, tui.
//!
//! This is the offensive "breaking LLMs for science" counterpart to Sentinel.
//! Local-first. Real effects. Loud personality. Fail loudly.
//!
//! See ARCHITECTURE.md and the design spec for boundaries.
//! Every layer has clear traits so they are independently testable (TDD).

pub mod bridge;
pub mod cli;
pub mod config;
pub mod event;
pub mod gadgets;
pub mod interceptor;
pub mod personality;
pub mod session;
pub mod tui;
pub mod watchlog;

// Re-exports for convenience at the top level.
pub use event::{Event, GhostFaceState, PersonalityHint};
pub use gadgets::Gadget;
pub use interceptor::{CommandWrapper, TcpTeeProxy};
pub use personality::PersonalityEngine;
pub use session::{Session, SessionMetrics};

// Trivial structure test for the skeleton init (TDD even on setup).
// Exercises that modules are present, types connect, and basic flows work.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gadgets::PokeGadget;
    use std::time::Instant;

    #[test]
    fn ghost_skeleton_structure_holds() {
        // can construct the main pieces
        let _engine = PersonalityEngine::new();
        let _ic = interceptor::Interceptor::new("test");
        let mut sess = Session::new("./some-agent");
        let poke = PokeGadget;
        assert_eq!(poke.name(), "poke");

        // round trip a fake event through session (exercises gadget + personality + event)
        let ev = Event::ToolCall {
            name: "test_tool".into(),
            args: "{}".into(),
            ts: Instant::now(),
        };
        sess.ingest(ev);
        assert!(sess.roast_count > 0);
        assert!(sess.summary().contains("some-agent") || sess.summary().contains("roasts:"));

        // config roundtrips
        let cfg = config::GhostConfig::with_defaults();
        assert!(!cfg.gadgets.is_empty());
    }
}

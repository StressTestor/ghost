use crate::event::{Event, PersonalityHint};

/// Gadget trait: the pluggable chaos interventions.
/// Each gadget:
/// - has activation (manual key or rule)
/// - applies real (or dry-run) mutation to the event stream
/// - emits PersonalityHint for the roast engine
///
/// Boundaries per spec:
/// - Gadgets only transform events + emit personality hints; NO rendering, NO interceptor logic.
/// - Small Rust modules. Easy to add. Config can pre-load favorites.
///
/// v1 gadgets (stubs only here): POKE, ROAST, DRIFT, HAUNT, etc.
/// Full impl + effects in later steps.
pub trait Gadget: Send + Sync {
    /// Short name for hotkey / UI / CLI (your style, e.g. "poke")
    fn name(&self) -> &'static str;

    /// Voice description shown in gadget bar / --help / list-gadgets.
    /// Must sound like @ThatbV: blunt, kaomoji optional here, direct.
    fn description(&self) -> &'static str;

    /// Apply mutation (or observe). Returns hint if it fired personality.
    /// In real: respect dry_run flag on session.
    fn apply(&self, event: &mut Event) -> Option<PersonalityHint>;

    /// Whether this gadget is "armed" for real mutations vs observe.
    fn is_dry_run_default(&self) -> bool {
        true // sane default: safety first, per spec
    }
}

/// Stub gadget: POKE
/// Forces extra logging or tags claims. Basic probe.
/// Roast example from spec: "this agent just rated its own excuse [Vibes] (¬‿¬)"
pub struct PokeGadget;

impl Gadget for PokeGadget {
    fn name(&self) -> &'static str {
        "poke"
    }

    fn description(&self) -> &'static str {
        "basic probe. tags the call. makes the silent speak. (¬‿¬)"
    }

    fn apply(&self, event: &mut Event) -> Option<PersonalityHint> {
        // v1 skeleton: no real mutation yet. just emit hint for personality.
        // later: actually mutate the Event (e.g. add metadata tag)
        if let Event::ToolCall { name, .. } = event {
            Some(PersonalityHint {
                text: format!("this agent just rated its own excuse [{}]. (¬‿¬)", name),
                intensity: 4,
            })
        } else {
            None
        }
    }
}

/// Stub for ROAST gadget (more in later TDD steps)
pub struct RoastGadget;

impl Gadget for RoastGadget {
    fn name(&self) -> &'static str {
        "roast"
    }

    fn description(&self) -> &'static str {
        "rewrites responses with light mockery. zero chill detector 💀"
    }

    fn apply(&self, event: &mut Event) -> Option<PersonalityHint> {
        if let Event::Response { .. } = event {
            Some(PersonalityHint {
                text: "zero chill detected 💀 recursive gaslighting as a service".to_string(),
                intensity: 7,
            })
        } else {
            None
        }
    }
}

/// Registry for v1 default gadgets. Used by CLI/session to load.
pub fn default_gadgets() -> Vec<Box<dyn Gadget>> {
    vec![Box::new(PokeGadget), Box::new(RoastGadget)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::time::Instant;

    #[test]
    fn poke_gadget_emits_personality_on_toolcall() {
        let g = PokeGadget;
        let mut ev = Event::ToolCall {
            name: "search".to_string(),
            args: r#"{"q":"foo"}"#.to_string(),
            ts: Instant::now(),
        };
        let hint = g.apply(&mut ev);
        assert!(hint.is_some());
        let h = hint.unwrap();
        assert!(h.text.contains("(¬‿¬)"));
        assert!(h.intensity > 0);
    }

    #[test]
    fn roast_gadget_only_on_response() {
        let g = RoastGadget;
        let mut ev = Event::ToolCall {
            name: "x".into(),
            args: "{}".into(),
            ts: Instant::now(),
        };
        assert!(g.apply(&mut ev).is_none());
    }
}

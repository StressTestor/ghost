use crate::event::{Event, PersonalityHint};

/// Centralized roast / personality engine.
/// Produces lines EXACTLY in @ThatbV X voice:
/// - kaomoji mandatory: >:[ (¬‿¬) (｡◕‿↼) 👻 💀 XX lmao
/// - blunt roasts, "zero chill", "they ALL talk eventually"
/// - mix security research directness + manic glee
/// - stream-of-consciousness where it fits
/// - anti-corporate, no hedging, never corporate voice
///
/// Used by: live log, ghost face state, session reports, headless output.
/// This is the single source of truth for "how ghost talks".
pub struct PersonalityEngine {
    // v1: stateless stub. later: prefs, rng for variation
}

impl PersonalityEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate a roast line from event + gadget context.
    /// Examples baked in for skeleton (will be used in TUI, reports).
    pub fn generate(&self, event: &Event, gadget_name: &str) -> String {
        match event {
            Event::ToolCall { name, .. } => {
                if gadget_name == "poke" {
                    format!(
                        "this agent just poked its own excuse for [{}]. (¬‿¬) they ALL talk eventually XX",
                        name
                    )
                } else {
                    format!("saw a {} on {}. zero chill detected 💀", gadget_name, name)
                }
            }
            Event::Response { .. } => {
                "response mutated. the worst kind of bug... everything reports success and nothing happens >:[ lmao".to_string()
            }
            _ => "digital bully mode engaged 👻 fuck off pete energy".to_string(),
        }
    }

    /// Turn a gadget's apply result into final personality line + face hint.
    pub fn from_hint(&self, hint: &PersonalityHint, _event: &Event) -> String {
        // v1: just use the text, but in real would enhance with kaomoji etc.
        // here we ensure the voice is injected at gadget level already.
        hint.text.clone()
    }
}

impl Default for PersonalityEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::time::Instant;

    #[test]
    fn personality_produces_voice_lines() {
        let engine = PersonalityEngine::new();
        let ev = Event::ToolCall {
            name: "search".into(),
            args: "{}".into(),
            ts: Instant::now(),
        };
        let line = engine.generate(&ev, "poke");
        assert!(line.contains("they ALL talk eventually"));
        assert!(line.contains("(¬‿¬)"));
    }
}

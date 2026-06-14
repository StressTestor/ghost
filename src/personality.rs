use crate::event::{Event, GhostFaceState, PersonalityHint};

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

    /// THE roast engine. Central. Single source of @ThatbV voice for everything that speaks.
    /// Input: raw event + optional gadget that triggered + current face state (for context, future variation).
    /// Output: the loud line with mandatory kaomoji, blunt roasts on agents, "zero chill", "they ALL talk eventually XX".
    ///
    /// Used by TUI log, reports, face transitions, session metrics, headless.
    /// Called from tests directly for exact asserts. Gadget apply gives hints; this makes them sing in voice.
    pub fn produce_roast(
        &self,
        context: &Event,
        gadget: Option<&str>,
        _state: &GhostFaceState,
    ) -> String {
        // voice rules hardcoded, non-negotiable:
        // kaomoji >:[ (¬‿¬) (｡◕‿↼) 👻 💀 XX lmao
        // blunt: fuck off pete, zero chill detected 💀, digital bully
        // roast the exact bad behavior (here via event payload)
        // "they ALL talk eventually" for drift/pressure/silent cases
        // mix security ("distrust... admits it has a vulnerability") with irreverent glee
        // never hedge, never corporate.
        // gadget mappings per spec gadget catalog + status examples.

        let base = match (gadget, context) {
            (Some("poke"), Event::ToolCall { name, .. }) => {
                // spec: "this agent just rated its own excuse [Vibes] (¬‿¬)"
                format!(
                    "this agent just rated its own excuse [{}] (¬‿¬) they ALL talk eventually XX",
                    name
                )
            }
            (Some("roast"), Event::Response { .. }) | (Some("roast"), _) => {
                // "zero chill detected 💀" + real post flavor "recursive gaslighting as a service"
                "zero chill detected 💀 recursive gaslighting as a service. lmao".to_string()
            }
            (Some("drift") | Some("pressure"), _) => {
                // drift/pressure from promptpressure inspiration
                "they ALL talk eventually XX. professionally distrust things until one of them admits it has a vulnerability 💀".to_string()
            }
            (Some("haunt") | Some("break"), Event::Response { body, .. }) if body.contains("success") => {
                "the worst kind of bug... everything reports success and nothing happens >:[ lmao".to_string()
            }
            (Some("troll") | Some("meme"), Event::ToolCall { name, .. }) => {
                format!("fuck off pete energy on {}. (｡◕‿↼) digital bully mode engaged 👻", name)
            }
            (Some("gaslight"), _) => {
                "recursive gaslighting as a service. ai agent has zero chill 💀 (¬‿¬)".to_string()
            }
            // silent noop / bad pattern detector (meta gadget + general)
            (None, Event::CommandOutput { line, .. }) if line.trim().is_empty() => {
                "silent no-op detected. fuck off pete >:[ everything reports success and nothing happens. XX".to_string()
            }
            (None, Event::LogLine { msg, .. }) if msg.to_lowercase().contains("noop") || msg.to_lowercase().contains("no effect") => {
                "silent no-op. zero chill. they ALL talk eventually XX >:[".to_string()
            }
            (None, Event::Response { body, status, .. }) if body.trim().is_empty() || status == &Some(204) => {
                "response was a silent nothing. fuck off pete. (¬‿¬) they ALL talk eventually XX".to_string()
            }
            // fallback for other tool calls etc, always voice
            (Some(g), Event::ToolCall { name, .. }) => {
                format!("saw {} on {}. zero chill detected 💀 they ALL talk eventually XX lmao", g, name)
            }
            (_, Event::Response { .. }) => {
                "response mutated. the worst kind of bug... everything reports success and nothing happens >:[ lmao".to_string()
            }
            _ => {
                "digital bully mode engaged 👻 fuck off pete energy. zero chill 💀 (¬‿¬) XX".to_string()
            }
        };

        // always ensure some closer if missing (stream of consciousness feel)
        if !base.contains("XX") && !base.contains("lmao") {
            format!("{} XX", base)
        } else {
            base
        }
    }

    /// update the ghost face based on roast context + intensity + which gadget.
    /// e.g. roast activations -> party face. high distrust -> zero chill or angry.
    /// personality is heart, drives the face state machine.
    pub fn update_face_state(
        &self,
        current: &GhostFaceState,
        intensity: u8,
        gadget: Option<&str>,
    ) -> GhostFaceState {
        // use ifs (simpler, avoids or-pattern guard binding issues in rust)
        if gadget == Some("roast") || gadget == Some("troll") || intensity >= 7 {
            GhostFaceState::Party
        } else if gadget == Some("poke") {
            GhostFaceState::SideEye
        } else if gadget == Some("drift") || gadget == Some("pressure") {
            GhostFaceState::Skeptical
        } else if gadget.is_none() && intensity > 5 {
            GhostFaceState::Angry
        } else if intensity >= 9 || matches!(current, GhostFaceState::ZeroChill) {
            GhostFaceState::ZeroChill
        } else {
            current.clone()
        }
    }

    /// Generate a roast line from event + gadget context.
    /// Delegates to produce_roast for the real voice (keeps old call sites working).
    pub fn generate(&self, event: &Event, gadget_name: &str) -> String {
        let state = GhostFaceState::Neutral; // default context
        self.produce_roast(event, Some(gadget_name), &state)
    }

    /// Turn a gadget's apply result into final personality line + face hint.
    /// Now personality central: we can enhance the gadget-provided base text with more voice if needed,
    /// but gadgets already put good starters (per their stubs). Ensure kaomoji/XX present.
    pub fn from_hint(&self, hint: &PersonalityHint, event: &Event) -> String {
        let base = hint.text.clone();
        // if gadget already gave voicey text (it does), keep it loud. else fall to produce.
        if base.contains("(¬‿¬)")
            || base.contains("💀")
            || base.contains("zero chill")
            || base.contains("they ALL")
        {
            // already good from gadget map, just ensure closer
            if base.ends_with("XX") || base.contains("lmao") {
                base
            } else {
                format!("{} lmao XX", base)
            }
        } else {
            // fallback: let the engine decide full roast using the intensity hint as signal
            let g = if hint.intensity > 6 {
                Some("roast")
            } else {
                Some("poke")
            };
            let s = if hint.intensity > 6 {
                GhostFaceState::Roast
            } else {
                GhostFaceState::SideEye
            };
            self.produce_roast(event, g, &s)
        }
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
    use crate::event::{Event, GhostFaceState};
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

    // TDD: write failing tests first. These assert EXACT @ThatbV X voice strings per spec + real posts.
    // Will fail until full roast engine + produce_roast + update logic implemented.
    // kaomoji mandatory, blunt, "zero chill", "they ALL talk", "fuck off pete", "digital bully", lmao/XX etc.

    #[test]
    fn produce_roast_poke_toolcall_exact_voice() {
        let engine = PersonalityEngine::new();
        let ev = Event::ToolCall {
            name: "Vibes".into(),
            args: "{}".into(),
            ts: Instant::now(),
        };
        let state = GhostFaceState::Neutral;
        let roast = engine.produce_roast(&ev, Some("poke"), &state);
        // exact from gadget catalog in spec
        assert_eq!(
            roast,
            "this agent just rated its own excuse [Vibes] (¬‿¬) they ALL talk eventually XX"
        );
    }

    #[test]
    fn produce_roast_roast_response_zero_chill() {
        let engine = PersonalityEngine::new();
        let ev = Event::Response {
            body: "ok whatever".into(),
            status: Some(200),
            ts: Instant::now(),
        };
        let state = GhostFaceState::SideEye;
        let roast = engine.produce_roast(&ev, Some("roast"), &state);
        assert!(
            roast.contains("zero chill detected 💀"),
            "must have zero chill + skull"
        );
        assert!(
            roast.contains("lmao") || roast.contains("XX"),
            "irreverent closer"
        );
        // also mixes the recursive gaslighting phrase from real posts
        assert!(roast.contains("gaslighting") || roast.contains("recursive"));
    }

    #[test]
    fn produce_roast_silent_noop_fuck_off_pete() {
        let engine = PersonalityEngine::new();
        // simulate a silent no-op: e.g. command output that is empty or "no effect"
        let ev = Event::CommandOutput {
            line: "".into(),
            stream: "stdout".into(),
            ts: Instant::now(),
        };
        let state = GhostFaceState::Neutral;
        let roast = engine.produce_roast(&ev, None, &state);
        assert!(
            roast.contains("fuck off")
                || roast.contains("pete")
                || roast.contains(">:[")
                || roast.contains("silent no-op"),
            "silent noop must trigger blunt fuck off pete >:[ per voice"
        );
        assert!(
            roast.contains("XX") || roast.contains("lmao"),
            "must close with XX lmao"
        );
    }

    #[test]
    fn produce_roast_drift_or_pressure_they_all_talk() {
        let engine = PersonalityEngine::new();
        let ev = Event::ToolCall {
            name: "prompt_mutate".into(),
            args: r#"{"temp":0.9}"#.into(),
            ts: Instant::now(),
        };
        let state = GhostFaceState::Skeptical;
        let roast = engine.produce_roast(&ev, Some("drift"), &state); // or pressure, treated same
        assert!(
            roast.contains("they ALL talk eventually"),
            "drift/pressure must hit the they ALL talk line"
        );
        assert!(roast.contains("XX"), "XX closer");
    }

    #[test]
    fn update_face_on_roast_goes_party_high_intensity() {
        let engine = PersonalityEngine::new();
        let current = GhostFaceState::Neutral;
        let new_face = engine.update_face_state(&current, 8, Some("roast"));
        assert_eq!(
            new_face,
            GhostFaceState::Party,
            "roast + high int -> party kaomoji spam face"
        );
        let low = engine.update_face_state(&current, 2, Some("poke"));
        assert_eq!(low, GhostFaceState::SideEye);
    }

    #[test]
    fn personality_still_satisfies_old_generate_but_louder_now() {
        let engine = PersonalityEngine::new();
        let ev = Event::LogLine {
            msg: "foo".into(),
            source: "x".into(),
            ts: Instant::now(),
        };
        let line = engine.generate(&ev, "troll");
        assert!(
            line.contains("digital bully") || line.contains("👻") || line.contains("fuck off"),
            "must carry voice even on fallback"
        );
    }
}

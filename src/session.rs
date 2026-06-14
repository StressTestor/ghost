use crate::event::{Event, GhostFaceState};
use crate::gadgets::{Gadget, default_gadgets};
use crate::personality::PersonalityEngine;

/// Session owns a live interception run.
/// Tracks events, state (chaos level, distrust metrics in your terms), gadget activations.
///
/// v1: simple in-memory. later: recording to disk (Vec<Event> + Vec<personality lines>)
/// Safety: dry-run default. explicit opt-in for real mutations.
pub struct Session {
    pub target: String,
    pub events: Vec<Event>,
    pub active_gadgets: Vec<Box<dyn Gadget>>,
    pub personality: PersonalityEngine,
    pub roast_count: usize,
    pub mutations_applied: usize,
    pub dry_run: bool,
    // basic event bus + state per spec (distrust + face for TUI/react)
    pub distrust_score: usize,
    pub ghost_face_state: GhostFaceState,
}

impl Session {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            events: Vec::new(),
            active_gadgets: default_gadgets(),
            personality: PersonalityEngine::new(),
            roast_count: 0,
            mutations_applied: 0,
            dry_run: true, // always start safe
            distrust_score: 0,
            ghost_face_state: GhostFaceState::Neutral,
        }
    }

    /// Ingest an event from interceptor. Apply armed gadgets. Record personality.
    /// This is the core event bus flow for skeleton.
    /// Also updates basic bus state: distrust_score + ghost_face_state on activity (for personality/react).
    pub fn ingest(&mut self, mut event: Event) {
        for gadget in &self.active_gadgets {
            if let Some(hint) = gadget.apply(&mut event) {
                let line = self.personality.from_hint(&hint, &event);
                // in real TUI: push to live log + face update
                // for skeleton: just count + stash a synthetic log event? keep simple.
                self.roast_count += 1;
                self.distrust_score += hint.intensity as usize;
                self.ghost_face_state = self.ghost_face_state.clone().on_roast(hint.intensity);
                if !self.dry_run {
                    self.mutations_applied += 1;
                }
                // emit side log line for demo (personality baked)
                self.events.push(Event::LogLine {
                    msg: line,
                    source: format!("gadget:{}", gadget.name()),
                    ts: event.ts(),
                });
            }
        }
        self.events.push(event);
    }

    pub fn summary(&self) -> String {
        format!(
            "session on {} | events: {} | roasts: {} | mutations: {} (dry_run={})",
            self.target,
            self.events.len(),
            self.roast_count,
            self.mutations_applied,
            self.dry_run
        )
    }

    /// Compat alias for event bus (some TDD paths use ingest_event).
    pub fn ingest_event(&mut self, event: Event) {
        self.ingest(event);
    }

    /// Force activate a named gadget (bypasses event match for test/demo).
    /// Uses a synthetic response for roast-style, or tool for poke. Updates bus state.
    /// Part of basic event bus control.
    pub fn activate_gadget(&mut self, name: &str) {
        // find matching gadget (case sensitive per v1)
        if let Some(g) = self.active_gadgets.iter().find(|g| g.name() == name) {
            // pick event type that will trigger the gadget's apply (roast on Response, poke on ToolCall)
            let mut ev = if name == "roast" {
                Event::Response {
                    body: "activate".into(),
                    status: Some(200),
                    ts: std::time::Instant::now(),
                }
            } else {
                Event::ToolCall {
                    name: "synthetic".into(),
                    args: "{}".into(),
                    ts: std::time::Instant::now(),
                }
            };
            if let Some(hint) = g.apply(&mut ev) {
                let line = self.personality.from_hint(&hint, &ev);
                self.roast_count += 1;
                self.distrust_score += hint.intensity as usize;
                self.ghost_face_state = self.ghost_face_state.clone().on_roast(hint.intensity);
                if !self.dry_run {
                    self.mutations_applied += 1;
                }
                self.events.push(Event::LogLine {
                    msg: line,
                    source: format!("gadget:{}", g.name()),
                    ts: ev.ts(),
                });
            }
        }
    }

    /// Return snapshot of bus metrics (roasts, distrust, current face state).
    /// Used by TUI / headless / tests. Simple struct for v1.
    pub fn get_metrics(&self) -> SessionMetrics {
        SessionMetrics {
            roast_count: self.roast_count,
            distrust_score: self.distrust_score,
            face: self.ghost_face_state.clone(),
        }
    }

    /// Wire basic event bus: take events emitted by interceptor (wrapper or proxy) and ingest all.
    /// Respects this session's dry_run for mutation counting. Safety: no auto mutate in interceptor itself.
    pub fn attach_with_interceptor(&mut self, events: Vec<Event>) {
        for e in events {
            self.ingest(e);
        }
    }
}

/// Simple metrics struct for get_metrics (v1 bus visibility, no overengineer).
#[derive(Debug, Clone, PartialEq)]
pub struct SessionMetrics {
    pub roast_count: usize,
    pub distrust_score: usize,
    pub face: GhostFaceState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::time::Instant;

    #[test]
    fn session_ingest_applies_gadgets_and_personality() {
        let mut s = Session::new("test-target");
        let ev = Event::ToolCall {
            name: "foo".into(),
            args: "{}".into(),
            ts: Instant::now(),
        };
        s.ingest(ev);
        // poke should have fired and added a LogLine roast
        assert!(s.roast_count >= 1);
        assert!(s.events.len() >= 2); // original + personality log
        assert!(s.summary().contains("roasts:"));
    }

    // TDD red first: these will fail until we enhance struct + add methods per spec/task
    #[test]
    fn session_tracks_distrust_and_ghost_face_state_on_ingest() {
        let mut s = Session::new("distrust-target");
        assert_eq!(s.distrust_score, 0);
        assert_eq!(s.ghost_face_state, crate::event::GhostFaceState::Neutral);

        let ev = Event::ToolCall {
            name: "risky_tool".into(),
            args: "{}".into(),
            ts: Instant::now(),
        };
        s.ingest(ev);
        // after poke fires, should have bumped distrust and changed face
        assert!(
            s.distrust_score >= 1,
            "distrust must rise on gadget activity"
        );
        assert_ne!(
            s.ghost_face_state,
            crate::event::GhostFaceState::Neutral,
            "face should react"
        );
    }

    #[test]
    fn session_ingest_event_and_activate_gadget_and_get_metrics() {
        let mut s = Session::new("metrics-target");
        let ev = Event::Response {
            body: "success".into(),
            status: Some(200),
            ts: Instant::now(),
        };
        s.ingest_event(ev); // new method per task
        s.activate_gadget("roast"); // should emit roast line + update state

        let m = s.get_metrics();
        assert!(m.roast_count >= 1);
        assert!(m.distrust_score >= 1);
        // on roast activate expect party face per example
        assert_eq!(m.face, crate::event::GhostFaceState::Party);
        assert!(s.summary().contains("roasts:"));
    }
}

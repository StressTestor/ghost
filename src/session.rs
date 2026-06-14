use crate::event::Event;
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
        }
    }

    /// Ingest an event from interceptor. Apply armed gadgets. Record personality.
    /// This is the core event bus flow for skeleton.
    pub fn ingest(&mut self, mut event: Event) {
        for gadget in &self.active_gadgets {
            if let Some(hint) = gadget.apply(&mut event) {
                let line = self.personality.from_hint(&hint, &event);
                // in real TUI: push to live log + face update
                // for skeleton: just count + stash a synthetic log event? keep simple.
                self.roast_count += 1;
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
}

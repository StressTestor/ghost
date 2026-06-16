use crate::event::{Event, GhostFaceState, RecordedEvent};
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
    /// collected personality roasts + attached banners for replay / headless print. voice baked.
    pub personality_lines: Vec<String>,
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
            personality_lines: Vec::new(),
        }
    }

    /// Ingest an event from interceptor. Apply armed gadgets. Record personality.
    /// This is the core event bus flow for skeleton.
    /// Also updates basic bus state: distrust_score + ghost_face_state on activity (for personality/react).
    pub fn ingest(&mut self, mut event: Event) {
        for gadget in &self.active_gadgets {
            if let Some(hint) = gadget.apply(&mut event, self.dry_run) {
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
                    msg: line.clone(),
                    source: format!("gadget:{}", gadget.name()),
                    ts: event.ts(),
                });
                self.personality_lines.push(line);
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
            if let Some(hint) = g.apply(&mut ev, self.dry_run) {
                let line = self.personality.from_hint(&hint, &ev);
                self.roast_count += 1;
                self.distrust_score += hint.intensity as usize;
                self.ghost_face_state = self.ghost_face_state.clone().on_roast(hint.intensity);
                if !self.dry_run {
                    self.mutations_applied += 1;
                }
                self.events.push(Event::LogLine {
                    msg: line.clone(),
                    source: format!("gadget:{}", g.name()),
                    ts: ev.ts(),
                });
                self.personality_lines.push(line);
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

    /// Select/arm only the named gadgets from the cli --gadgets or config list.
    /// If empty or "all", keeps defaults. Respects existing trait logic (no change to apply).
    /// Used by main wiring for attach/run.
    pub fn select_gadgets(&mut self, names: &[String]) {
        if names.is_empty() || names.iter().any(|n| n == "all" || n == "default") {
            return; // keep loaded defaults
        }
        let wanted: std::collections::HashSet<String> =
            names.iter().map(|s| s.to_lowercase()).collect();
        self.active_gadgets.retain(|g| wanted.contains(g.name()));
        // if filter removed all (bad list), fall back to defaults for safety/voice
        if self.active_gadgets.is_empty() {
            self.active_gadgets = default_gadgets();
        }
    }

    /// basic recording: save personality_lines (roasts + banners in exact voice) + fallback events to txt file.
    /// id used for filename ghost-recording-<id>.txt . returns the full path for replay cmd + print.
    /// called after attach/proxy/run exit. simple, no new deps.
    pub fn save_recording(&self, id: &str) -> std::io::Result<String> {
        let dir = recordings_dir();
        ensure_private_dir(&dir)?;
        let path = dir.join(format!("ghost-recording-{}.txt", id));
        let content = if !self.personality_lines.is_empty() {
            self.personality_lines.join("\n")
        } else {
            // fallback: pull LogLines that carry voice (banners/roasts)
            self.events
                .iter()
                .filter_map(|e| {
                    if let Event::LogLine { msg, .. } = e {
                        if msg.contains("👻")
                            || msg.contains("zero chill")
                            || msg.contains("they ALL")
                            || msg.contains("fuck off")
                            || msg.contains("XX")
                            || msg.contains("lmao")
                            || msg.contains("(¬")
                            || msg.contains("💀")
                        {
                            Some(msg.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        std::fs::write(&path, &content)?;
        Ok(path.display().to_string())
    }

    /// project the live events into their serializable form (recording t=0 at
    /// the first event, so relative timing survives the Instant->disk gap).
    pub fn to_recorded_events(&self) -> Vec<RecordedEvent> {
        let Some(first) = self.events.first() else {
            return Vec::new();
        };
        let first_ts = first.ts();
        self.events
            .iter()
            .enumerate()
            .map(|(i, e)| RecordedEvent::from_event(e, i, first_ts))
            .collect()
    }

    /// STRUCTURED recording: one RecordedEvent per line (JSONL). unlike the voice
    /// .txt (which is for replay vibes), this is a real machine-readable trace —
    /// the thing the README means by "feed to your evals". returns the path.
    pub fn save_recording_jsonl(&self, id: &str) -> std::io::Result<String> {
        let dir = recordings_dir();
        ensure_private_dir(&dir)?;
        let path = dir.join(format!("ghost-recording-{}.jsonl", id));
        let body = self
            .to_recorded_events()
            .iter()
            .map(|r| r.to_jsonl())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, body)?;
        Ok(path.display().to_string())
    }
}

/// where recordings live: `~/.ghost/recordings`, falling back to the cwd only if
/// there's no HOME. recordings (esp. the .jsonl) capture RAW command output, so
/// they belong in ghost's private dir, NOT in whatever repo you ran `attach`
/// from. does not create the dir — see `ensure_private_dir`.
pub fn recordings_dir() -> std::path::PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => std::path::Path::new(&home)
            .join(".ghost")
            .join("recordings"),
        None => std::path::PathBuf::from("."),
    }
}

/// create `dir` (recursively) and lock it to the owner (0700 on unix) since it
/// holds captured command output. perms are best-effort.
fn ensure_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
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
    fn save_recording_jsonl_writes_a_parseable_structured_trace() {
        use crate::event::RecordedEvent;
        let mut s = Session::new("jsonl-rec");
        s.ingest(Event::ToolCall {
            name: "Read".into(),
            args: "{}".into(),
            ts: Instant::now(),
        });
        s.ingest(Event::Response {
            body: "ok".into(),
            status: Some(200),
            ts: Instant::now(),
        });

        let path = s
            .save_recording_jsonl("unit-jsonl-1781400000")
            .expect("write jsonl");
        let content = std::fs::read_to_string(&path).expect("read back");
        let recs: Vec<RecordedEvent> = content
            .lines()
            .filter_map(RecordedEvent::from_jsonl)
            .collect();

        // every event projected, one per line, in order, all parseable
        assert_eq!(
            recs.len(),
            s.events.len(),
            "one structured record per live event"
        );
        assert!(recs.len() >= 2);
        // seq is sequential from 0
        for (i, r) in recs.iter().enumerate() {
            let seq = match r {
                RecordedEvent::ToolCall { seq, .. }
                | RecordedEvent::Response { seq, .. }
                | RecordedEvent::CommandOutput { seq, .. }
                | RecordedEvent::Log { seq, .. } => *seq,
            };
            assert_eq!(seq, i, "seq must match position");
        }

        let _ = std::fs::remove_file(&path);
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

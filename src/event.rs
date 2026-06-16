use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Core event types for the interception stream.
/// All gadgets and renderer subscribe to these.
/// (real targets, real effects -- no game)
#[derive(Debug, Clone)]
pub enum Event {
    /// Agent/tool call with name + raw args (as seen on wire)
    ToolCall {
        name: String,
        args: String,
        ts: Instant,
    },
    /// Response from the target (body + optional status)
    Response {
        body: String,
        status: Option<u16>,
        ts: Instant,
    },
    /// Raw command / stdout/stderr line capture
    CommandOutput {
        line: String,
        stream: String, // "stdout" | "stderr"
        ts: Instant,
    },
    /// Log or side channel line (from tailer etc)
    LogLine {
        msg: String,
        source: String,
        ts: Instant,
    },
    // future: Http etc. YAGNI for v1 skeleton
}

/// Hint returned by gadgets to feed the personality/roast engine.
/// Carries the spooky line + intensity for face + effects.
#[derive(Debug, Clone)]
pub struct PersonalityHint {
    pub text: String,
    pub intensity: u8, // 0-10, drives ghost face etc
}

/// Serializable projection of an `Event` for on-disk recordings (JSONL).
/// `Event` itself stays on `Instant` (monotonic, right for the live model but
/// not serializable across runs); this captures a recording-friendly view:
/// `seq` for order, `t_ms` for relative timing (ms since the first event), and
/// the payload. one `RecordedEvent` per line = a structured trace you can
/// actually feed to evals, not just voice .txt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum RecordedEvent {
    ToolCall {
        seq: usize,
        t_ms: u64,
        name: String,
        args: String,
    },
    Response {
        seq: usize,
        t_ms: u64,
        body: String,
        status: Option<u16>,
    },
    CommandOutput {
        seq: usize,
        t_ms: u64,
        line: String,
        stream: String,
    },
    Log {
        seq: usize,
        t_ms: u64,
        msg: String,
        source: String,
    },
}

impl RecordedEvent {
    /// project a live `Event` into its recordable form. `t_ms` is this event's
    /// offset from `first_ts` (the recording's t=0), so timing survives even
    /// though the absolute Instant doesn't.
    pub fn from_event(ev: &Event, seq: usize, first_ts: Instant) -> Self {
        let t_ms = ev.ts().saturating_duration_since(first_ts).as_millis() as u64;
        match ev {
            Event::ToolCall { name, args, .. } => RecordedEvent::ToolCall {
                seq,
                t_ms,
                name: name.clone(),
                args: args.clone(),
            },
            Event::Response { body, status, .. } => RecordedEvent::Response {
                seq,
                t_ms,
                body: body.clone(),
                status: *status,
            },
            Event::CommandOutput { line, stream, .. } => RecordedEvent::CommandOutput {
                seq,
                t_ms,
                line: line.clone(),
                stream: stream.clone(),
            },
            Event::LogLine { msg, source, .. } => RecordedEvent::Log {
                seq,
                t_ms,
                msg: msg.clone(),
                source: source.clone(),
            },
        }
    }

    /// one compact JSONL line (no newline). serialization of plain data can't
    /// fail; on the impossible chance it does, emit a minimal valid object.
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"kind":"Log","msg":"<unserializable>"}"#.to_string())
    }

    /// parse one JSONL line back; junk -> None (forgiving replay).
    pub fn from_jsonl(line: &str) -> Option<Self> {
        let t = line.trim();
        if t.is_empty() {
            return None;
        }
        serde_json::from_str(t).ok()
    }
}

impl Event {
    pub fn ts(&self) -> Instant {
        match self {
            Event::ToolCall { ts, .. } => *ts,
            Event::Response { ts, .. } => *ts,
            Event::CommandOutput { ts, .. } => *ts,
            Event::LogLine { ts, .. } => *ts,
        }
    }

    /// source per spec data model (agent/command/log etc). implicit from variant but explicit here.
    pub fn source(&self) -> &'static str {
        match self {
            Event::ToolCall { .. } => "agent/tool",
            Event::Response { .. } => "agent/response",
            Event::CommandOutput { .. } => "command",
            Event::LogLine { .. } => "log",
        }
    }
}

/// GhostFaceState - part of core data model.
/// Drives the TUI ghost face widget + effects. Intensity + expression.
/// Updated by personality roast engine on events/gadgets.
/// Exact kaomoji/faces from @ThatbV voice: 👻 (¬‿¬) >:[ (｡◕‿↼) 💀 ಠ‿ಠ
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GhostFaceState {
    #[default]
    Neutral, // 👻 default spooky
    SideEye,   // (¬‿¬) poke / observing
    Roast,     // (｡◕‿↼) good hit
    Angry,     // >:[ silent noop / bad pattern
    Party,     // kaomoji spam on roast activate / high intensity
    Skeptical, // ಠ‿ಠ distrust rising
    ZeroChill, // 💀 roast mode full
}

impl GhostFaceState {
    pub fn emoji(&self) -> &'static str {
        match self {
            GhostFaceState::Neutral => "👻",
            GhostFaceState::SideEye => "(¬‿¬)",
            GhostFaceState::Roast => "(｡◕‿↼)",
            GhostFaceState::Angry => ">:[",
            GhostFaceState::Party => "💀👻(¬‿¬)",
            GhostFaceState::Skeptical => "ಠ‿ಠ",
            GhostFaceState::ZeroChill => "💀",
        }
    }

    /// simple update rule, personality will drive more
    pub fn on_roast(&self, intensity: u8) -> Self {
        if intensity >= 7 {
            GhostFaceState::Party
        } else if intensity >= 5 {
            GhostFaceState::Roast
        } else if intensity >= 3 {
            GhostFaceState::SideEye
        } else {
            self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn event_creates_with_variants_and_source() {
        let ts = Instant::now();
        let tc = Event::ToolCall {
            name: "search".into(),
            args: r#"{"q":"x"}"#.into(),
            ts,
        };
        assert_eq!(tc.source(), "agent/tool");
        assert_eq!(tc.ts(), ts);

        let resp = Event::Response {
            body: "ok".into(),
            status: Some(200),
            ts,
        };
        assert_eq!(resp.source(), "agent/response");

        let cmd = Event::CommandOutput {
            line: "output".into(),
            stream: "stdout".into(),
            ts,
        };
        assert_eq!(cmd.source(), "command");
    }

    #[test]
    fn recorded_event_projects_each_variant_and_roundtrips() {
        let t0 = Instant::now();
        let tc = Event::ToolCall {
            name: "Read".into(),
            args: r#"{"p":"x"}"#.into(),
            ts: t0,
        };
        let rec = RecordedEvent::from_event(&tc, 0, t0);
        match &rec {
            RecordedEvent::ToolCall {
                seq,
                name,
                args,
                t_ms,
            } => {
                assert_eq!(*seq, 0);
                assert_eq!(name, "Read");
                assert_eq!(args, r#"{"p":"x"}"#);
                assert_eq!(*t_ms, 0, "first event is at t=0");
            }
            _ => panic!("wrong variant"),
        }
        // jsonl roundtrip
        let line = rec.to_jsonl();
        assert!(line.contains("\"kind\":\"ToolCall\""));
        assert!(!line.contains('\n'));
        assert_eq!(RecordedEvent::from_jsonl(&line), Some(rec));

        // other variants project cleanly too
        let resp = Event::Response {
            body: "ok".into(),
            status: Some(200),
            ts: t0,
        };
        assert!(matches!(
            RecordedEvent::from_event(&resp, 1, t0),
            RecordedEvent::Response {
                status: Some(200),
                seq: 1,
                ..
            }
        ));
        let log = Event::LogLine {
            msg: "hi".into(),
            source: "x".into(),
            ts: t0,
        };
        assert!(matches!(
            RecordedEvent::from_event(&log, 2, t0),
            RecordedEvent::Log { .. }
        ));
    }

    #[test]
    fn recorded_event_from_jsonl_is_forgiving() {
        assert!(RecordedEvent::from_jsonl("").is_none());
        assert!(RecordedEvent::from_jsonl("{not json").is_none());
        assert!(RecordedEvent::from_jsonl(r#"{"kind":"Nope"}"#).is_none());
    }

    #[test]
    fn ghost_face_state_default_and_emoji_and_on_roast() {
        let s: GhostFaceState = Default::default();
        assert_eq!(s, GhostFaceState::Neutral);
        assert_eq!(s.emoji(), "👻");

        let side = GhostFaceState::SideEye;
        assert_eq!(side.emoji(), "(¬‿¬)");

        let angry = GhostFaceState::Angry;
        assert_eq!(angry.emoji(), ">:[");

        // on_roast logic
        let after = GhostFaceState::Neutral.on_roast(8);
        assert_eq!(after, GhostFaceState::Party);
        let mid = GhostFaceState::Neutral.on_roast(4);
        assert_eq!(mid, GhostFaceState::SideEye);
    }
}

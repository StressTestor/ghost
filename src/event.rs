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

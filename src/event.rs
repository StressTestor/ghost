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
}

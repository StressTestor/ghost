use crate::event::Event;
use std::time::Instant;

/// Interceptor / Attachment layer.
/// The real boundary (inspired by Sentinel hooks).
///
/// Pluggable backends v1:
/// - Command wrapper (exec + stdin/stdout/stderr capture + hook injection)
/// - Simple HTTP/local process proxy (tokio)
/// - Log tail + parser
///
/// Boundaries (strict):
/// - Interceptor ONLY emits events; never knows about UI, gadgets, or personality.
/// - Never auto-mutates without explicit gadget + user confirmation (dry-run first).
/// - Clear "ghost is attached" banners on real targets.
/// - Exit paths always clean.
///
/// This is skeleton only. Real attach/proxy logic + wrapper in follow-on TDD.
pub struct Interceptor {
    // target info, backend state stubs
    target: String,
}

impl Interceptor {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }

    /// Start the interception. In real: spawns the wrapper or proxy.
    /// For skeleton: produces a couple fake events to exercise the bus.
    pub fn start(&self) -> Vec<Event> {
        // TODO later: actual process exec / tokio listener / tail
        vec![
            Event::LogLine {
                msg: format!("ghost 👻 attached to {}", self.target),
                source: "interceptor".into(),
                ts: Instant::now(),
            },
            Event::ToolCall {
                name: "stub_tool".into(),
                args: "{}".into(),
                ts: Instant::now(),
            },
        ]
    }

    pub fn target(&self) -> &str {
        &self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interceptor_emits_events_without_knowing_gadgets() {
        let ic = Interceptor::new("./my-agent");
        let events = ic.start();
        assert!(!events.is_empty());
        // important: no gadget types here. pure events.
        assert!(matches!(events[0], Event::LogLine { .. }));
    }
}

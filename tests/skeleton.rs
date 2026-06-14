// integration / structure smoke tests for the ghost skeleton.
// these exercise the public surface of the lib from outside src/.
// (real command wrapping + end-to-end attach tests come later with TDD.)

use ghost::cli::{Cli, Commands};
use ghost::gadgets::default_gadgets;
use ghost::session::Session;
use ghost::tui::TuiRenderer;
use ghost::{Event, PersonalityEngine};
use std::time::Instant;

#[test]
fn external_test_can_use_lib_types_and_gadgets() {
    // clap structs are public for testing
    let _cli = Cli {
        command: Commands::Gadgets,
    };

    let gadgets = default_gadgets();
    assert!(!gadgets.is_empty());
    for g in &gadgets {
        assert!(!g.name().is_empty());
        assert!(!g.description().is_empty());
    }

    let mut s = Session::new("external-test");
    let ev = Event::ToolCall {
        name: "external".into(),
        args: "{}".into(),
        ts: Instant::now(),
    };
    s.ingest(ev);
    assert!(s.roast_count > 0);
}

#[test]
fn tui_headless_path_works_without_terminal() {
    let s = Session::new("headless-check");
    let r = TuiRenderer::new();
    let out = r.headless_summary(&s);
    assert!(out.contains("ghost 👻 headless"));
    assert!(out.contains("roasts fired"));
}

#[test]
fn personality_engine_accessible_from_tests_dir() {
    let engine = PersonalityEngine::new();
    let ev = Event::Response {
        body: "ok".into(),
        status: Some(200),
        ts: Instant::now(),
    };
    let line = engine.generate(&ev, "roast");
    assert!(line.contains("zero chill") || line.contains("bug"));
}

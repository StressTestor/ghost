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

// integration-ish TDD tests from outside src/: exercise core model + full personality roast on simulated events.
// (real attach flows come later; this wires Event/Session/GhostFaceState/personality as the heart.)
#[test]
fn tests_dir_exercises_personality_roast_engine_on_sim_events() {
    // use reexported types (via lib.rs wire)
    let engine = PersonalityEngine::new();
    let mut s = Session::new("sim-integration");

    // toolcall + poke
    let poke_ev = Event::ToolCall {
        name: "search_api".into(),
        args: r#"{"q":"foo"}"#.into(),
        ts: Instant::now(),
    };
    s.ingest(poke_ev.clone());
    let roast1 = engine.produce_roast(&poke_ev, Some("poke"), &ghost::GhostFaceState::Neutral);
    assert!(
        roast1.contains("[search_api] (¬‿¬)"),
        "poke roast must use event name + kaomoji"
    );
    assert!(
        roast1.contains("they ALL talk eventually XX"),
        "core phrase"
    );

    // response + roast -> zero chill + face to party
    let resp_ev = Event::Response {
        body: "result".into(),
        status: Some(200),
        ts: Instant::now(),
    };
    s.activate_gadget("roast"); // exercises session + personality path
    let roast2 = engine.produce_roast(&resp_ev, Some("roast"), &s.ghost_face_state);
    assert!(
        roast2.contains("zero chill detected 💀"),
        "roast gadget exact voice"
    );
    assert!(
        roast2.contains("gaslighting"),
        "real X post phrase baked in"
    );

    let m = s.get_metrics();
    assert!(m.roast_count >= 2);
    assert_eq!(
        m.face,
        ghost::GhostFaceState::Party,
        "activate roast should drive party face via personality update"
    );
    assert!(m.distrust_score > 0);
}

#[test]
fn tests_dir_ghost_face_and_session_state_from_personality() {
    use ghost::GhostFaceState;
    let engine = PersonalityEngine::new();
    let ev = Event::CommandOutput {
        line: "".into(),
        stream: "stderr".into(),
        ts: Instant::now(),
    };
    let roast = engine.produce_roast(&ev, None, &GhostFaceState::Neutral);
    assert!(
        roast.contains("fuck off") || roast.contains("silent no-op") || roast.contains(">:["),
        "noop case hits blunt voice"
    );
    assert!(roast.contains("XX"));

    // direct face update
    let face = engine.update_face_state(&GhostFaceState::Neutral, 8, Some("roast"));
    assert_eq!(face, GhostFaceState::Party);
}

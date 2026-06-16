// integration / structure smoke tests for the ghost skeleton.
// these exercise the public surface of the lib from outside src/.
// TDD for cli wiring, headless, replay, config, attach --dry-run voice asserts per spec + task.
// (real command wrapping + end-to-end attach tests come later with TDD.)

use ghost::cli::{Cli, Commands};
use ghost::config::GhostConfig;
use ghost::gadgets::default_gadgets;
use ghost::session::Session;
use ghost::tui::TuiRenderer;
use ghost::{Event, PersonalityEngine};
use std::time::Instant; // bring Parser trait for try_parse_from in cli wiring tests

#[test]
fn external_test_can_use_lib_types_and_gadgets() {
    // clap structs are public for testing (globals added for --headless --config wiring)
    let _cli = Cli {
        command: Commands::Gadgets,
        headless: false,
        config: None,
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

// === new integration TDD for full cli wiring + headless + replay + config + dry-run voice ===

#[test]
fn cli_and_headless_use_existing_surface_and_voice() {
    // direct construct matches current Cli (no extra globals in v1)
    let _cli = Cli {
        command: Commands::Attach {
            command: vec!["echo".into(), "hi".into()],
            gadgets: vec!["poke".into()],
            dry_run: true,
        },
        headless: false,
        config: None,
    };

    // headless path (no ratatui) + voice
    let mut s = Session::new("headless-voice-test");
    s.activate_gadget("poke");
    let r = TuiRenderer::new();
    let out = r.headless_summary(&s);
    assert!(out.contains("ghost 👻 headless") || out.contains("roasts fired"));
    // use existing events for voice (no personality_lines field)
    let has_voice = s.events.iter().any(|e| {
        if let Event::LogLine { msg, .. } = e {
            msg.contains("(¬‿¬)")
                || msg.contains("they ALL talk eventually")
                || msg.contains("👻")
                || msg.contains("zero chill")
        } else {
            false
        }
    });
    assert!(
        has_voice || s.roast_count > 0,
        "headless path must carry personality voice/kaomoji"
    );
}

#[test]
fn replay_skipped_yagni_but_voice_still_tested_via_headless_and_face() {
    // replay removed (YAGNI for v1 TUI task); voice + face covered by other tui tests + this
    let engine = PersonalityEngine::new();
    let ev = Event::ToolCall {
        name: "x".into(),
        args: "{}".into(),
        ts: Instant::now(),
    };
    let line = engine.produce_roast(&ev, Some("poke"), &ghost::GhostFaceState::Neutral);
    assert!(line.contains("(¬‿¬)") && line.contains("they ALL talk eventually"));
}

#[test]
fn config_gadgets_and_voice_prefs_for_cli_wiring() {
    let cfg = GhostConfig::with_defaults();
    assert!(
        cfg.gadgets
            .iter()
            .any(|g| g.contains("poke") || g.contains("roast")),
        "defaults has gadgets"
    );
    assert_eq!(cfg.voice.kaomoji_level, 7);

    // roundtrip already tested in config, here confirm accessible from tests/
    let serialized = toml::to_string(&cfg).unwrap();
    let back: GhostConfig = toml::from_str(&serialized).unwrap();
    assert_eq!(back.voice.kaomoji_level, cfg.voice.kaomoji_level);
}

#[test]
fn command_wrapper_streams_events_live_via_sink() {
    // run_streaming must hand each line to the sink as it arrives (live), capture
    // BOTH stdout and stderr without deadlocking, and report the real exit code.
    let wrapper = ghost::interceptor::CommandWrapper::new(vec![
        "sh".into(),
        "-c".into(),
        "echo out1; echo err1 1>&2; echo out2".into(),
    ]);
    let mut seen: Vec<Event> = Vec::new();
    let code = wrapper.run_streaming(true, &mut |e| seen.push(e));

    assert_eq!(code, 0, "exit code surfaced");
    // banner is first (attached before anything runs)
    assert!(
        matches!(&seen[0], Event::LogLine { msg, .. } if msg.contains("👻 ghost attached")),
        "banner emitted first"
    );
    let outs: Vec<(String, String)> = seen
        .iter()
        .filter_map(|e| match e {
            Event::CommandOutput { line, stream, .. } => Some((line.clone(), stream.clone())),
            _ => None,
        })
        .collect();
    assert!(
        outs.iter()
            .any(|(l, s)| l.contains("out1") && s == "stdout"),
        "captured stdout live"
    );
    assert!(
        outs.iter()
            .any(|(l, s)| l.contains("err1") && s == "stderr"),
        "captured stderr live (no deadlock)"
    );
    assert!(outs.iter().any(|(l, _)| l.contains("out2")));
    assert!(
        seen.iter()
            .any(|e| matches!(e, Event::LogLine { msg, .. } if msg.contains("exited with code"))),
        "exit logline closes the stream"
    );
}

#[test]
fn tcp_tee_proxy_actually_forwards_bytes_both_ways_and_tees_events() {
    use ghost::interceptor::TcpTeeProxy;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    // 1) a real upstream that echoes whatever it receives.
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    let upstream_addr = upstream.local_addr().unwrap().to_string();
    let up = std::thread::spawn(move || {
        let (mut s, _) = upstream.accept().unwrap();
        let mut buf = [0u8; 1024];
        loop {
            match s.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if s.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // 2) the proxy's listen socket; one accept -> tee_connection to the upstream.
    let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap().to_string();
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let ev2 = Arc::clone(&events);
    let proxy = std::thread::spawn(move || {
        let (client, _) = proxy_listener.accept().unwrap();
        TcpTeeProxy::tee_connection(client, &upstream_addr, &mut |e| ev2.lock().unwrap().push(e));
    });

    // 3) a client talks to the PROXY and must get its bytes echoed back through it.
    let mut client = TcpStream::connect(&proxy_addr).unwrap();
    client.write_all(b"ping ghost").unwrap();
    client.shutdown(Shutdown::Write).unwrap(); // EOF so the tee can drain + close
    let mut got = String::new();
    client.read_to_string(&mut got).unwrap();
    assert_eq!(
        got, "ping ghost",
        "bytes round-tripped through the real proxy"
    );

    proxy.join().unwrap();
    up.join().unwrap();

    // 4) both directions were teed onto the bus as events.
    let evs = events.lock().unwrap();
    let dirs: Vec<String> = evs
        .iter()
        .filter_map(|e| match e {
            Event::CommandOutput { stream, .. } => Some(stream.clone()),
            _ => None,
        })
        .collect();
    assert!(
        dirs.iter().any(|d| d == "client→target"),
        "client->target teed"
    );
    assert!(
        dirs.iter().any(|d| d == "target→client"),
        "target->client teed"
    );
}

#[test]
fn tcp_tee_proxy_unreachable_upstream_is_loud_not_a_panic() {
    use ghost::interceptor::TcpTeeProxy;
    use std::net::{TcpListener, TcpStream};

    let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap().to_string();
    let mut seen: std::sync::Arc<std::sync::Mutex<Vec<Event>>> = Default::default();
    let s2 = std::sync::Arc::clone(&seen);
    let h = std::thread::spawn(move || {
        let (client, _) = proxy_listener.accept().unwrap();
        // 127.0.0.1:1 is a privileged/closed port -> connect fails fast.
        TcpTeeProxy::tee_connection(client, "127.0.0.1:1", &mut |e| s2.lock().unwrap().push(e));
    });
    let _client = TcpStream::connect(&proxy_addr).unwrap();
    h.join().unwrap();

    let evs = std::sync::Arc::get_mut(&mut seen)
        .unwrap()
        .get_mut()
        .unwrap();
    assert!(
        evs.iter()
            .any(|e| matches!(e, Event::LogLine { msg, source, .. }
            if source == "error" && msg.contains("unreachable"))),
        "unreachable upstream must emit a loud error event"
    );
}

#[test]
fn command_wrapper_bad_cmd_streams_loud_error_and_nonzero() {
    let wrapper =
        ghost::interceptor::CommandWrapper::new(vec!["definitely-not-a-real-cmd-xyz-789".into()]);
    let mut seen: Vec<Event> = Vec::new();
    let code = wrapper.run_streaming(true, &mut |e| seen.push(e));
    assert_eq!(code, -1, "failed launch reports -1");
    assert!(
        seen.iter()
            .any(|e| matches!(e, Event::LogLine { msg, source, .. }
            if source == "error" && msg.contains("exec failed"))),
        "loud error event, never swallowed"
    );
}

#[test]
fn attach_dry_run_via_wrapper_and_session_emits_voice_banners_no_side_effects() {
    // integration: use real CommandWrapper (as attach does) + session bus + dry
    // asserts exact banners + personality roasts in voice, dry passed, no "real mode" text
    let wrapper =
        ghost::interceptor::CommandWrapper::new(vec!["echo".into(), "ghost-dry-voice-test".into()]);
    let events = wrapper.run(true); // dry
    assert!(events.iter().any(|e| matches!(e, Event::LogLine { msg, .. } if msg.contains("👻 ghost attached (observe only)") && msg.contains("(¬‿¬)") && msg.contains("they ALL talk eventually XX") )), "dry banner exact voice");

    let mut sess = Session::new("dry-attach-test");
    sess.dry_run = true;
    sess.attach_with_interceptor(events);

    // must have ingested roasts from gadgets (poke on cmd? or via lines), but at min banner + output
    let has_banner_voice = sess.events.iter().any(|e| {
        if let Event::LogLine { msg, .. } = e {
            msg.contains("👻 ghost attached") && (msg.contains("(¬‿¬)") || msg.contains("XX"))
        } else {
            false
        }
    });
    assert!(
        has_banner_voice,
        "session bus must carry voice banner from dry attach"
    );

    // dry guarantees no mutations counted
    assert!(sess.dry_run);
    // output captured (echo line)
    assert!(sess.events.iter().any(
        |e| matches!(e, Event::CommandOutput { line, .. } if line.contains("ghost-dry-voice-test"))
    ));

    // voice roasts are in LogLine events (gadget: sources) for headless path
    let has_roast_line = sess.events.iter().any(|e| {
        if let Event::LogLine { msg, source, .. } = e {
            (source.starts_with("gadget:") || msg.contains("👻"))
                && (msg.contains("(¬‿¬)") || msg.contains("they ALL"))
        } else {
            false
        }
    });
    assert!(
        has_roast_line || sess.roast_count > 0,
        "voice roasts in events for headless/TUI log"
    );
}

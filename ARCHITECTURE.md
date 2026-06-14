# architecture

**ghost** 👻 cli + tui for live visibility + deliberate chaos injection into real agent tool calls, cli commands, local services. offensive research counterpart to sentinel. "they ALL talk eventually XX"

last updated: 2026-06-14 (full ratatui TUI: GhostFaceWidget (kaomoji per GhostFaceState 5-7 lines + intensity color/glitch), ActivityCanvas (glitchy event stream !! + invert on high/party), GadgetBar (hotkeys+voice descs), status strip (ZERO CHILL etc), live log tail, overlays (help/confirm in voice), main TUI loop (crossterm poll/keys/q/1-9/space/h, consume Session), headless (tty/env or flag, voice prints), TDD Buffer tests + full suite 50+; updated main wiring + skeleton; ARCH sync)

## project overview

one-liner: real daily-driver terminal tool (not a game) that watches your actual agents/commands/services and lets you inject scoped chaos with loud @ThatbV personality baked into every roast, face, and trace.

why: makes watching tool calls entertaining instead of soul-crushing. complements defense (sentinel) with offensive poking for hardening + "breaking llms for science". local-first. no cloud. fail loudly.

## stack and dependencies

| layer          | technology                  | version / notes                  |
|----------------|-----------------------------|----------------------------------|
| language       | rust                        | 1.96+ (edition 2024 in init)     |
| cli            | clap                        | 4 (with derive)                  |
| tui            | ratatui + crossterm         | 0.30 / 0.29                      |
| async          | tokio                       | 1 (full)                         |
| config         | serde + toml                | 1 + 1.1                          |
| randomness     | rand                        | 0.10                             |
| errors         | thiserror                   | 2                                |
| core           | std (instant, etc)          | -                                |
| release        | cargo profile               | lto=true, strip=true, codegen-units=1, opt-level=3 |

(no db. recordings are vec<event> + personality lines serialized later.)

## directory structure

```
ghost/
├── Cargo.toml
├── Cargo.lock
├── .gitignore
├── README.md
├── ARCHITECTURE.md
├── docs/
│   └── superpowers/
│       └── specs/
│           └── 2026-06-14-ghost-design.md   # source of truth for v1
├── src/
│   ├── main.rs          # thin clap dispatch + voice banners
│   ├── lib.rs           # module root + reexports + skeleton structure test
│   ├── cli.rs           # clap subcommands (attach, proxy, run, replay, gadgets)
│   ├── config.rs        # GhostConfig + VoicePrefs + toml load (serde)
│   ├── event.rs         # Event enum + PersonalityHint (core data model)
│   ├── interceptor.rs   # attachment backends (real v1: CommandWrapper using std::process for exec/capture of CommandOutput, ProxyStub). emits pure Events only. banners + dry_run safety. no gadgets/mutation here.
│   ├── session.rs       # owns live run, ingests events (core bus), applies gadgets, tracks roasts/mutations/distrust/face. attach_with_interceptor(events) wires wrapper output. SessionMetrics for visibility.
│   ├── personality.rs   # roast engine. single source of @ThatbV voice. kaomoji, blunt, "zero chill"
│   ├── gadgets/
│   │   └── mod.rs       # Gadget trait + stubs (PokeGadget, RoastGadget) + registry. apply returns hint
│   ├── tui.rs           # full ratatui TUI per spec: widgets (GhostFace 5-7 lines kaomoji+colors+effects, Activity glitchy stream, GadgetBar voice names/hotkeys, Status "ZERO CHILL...", LiveLog), crossterm loop (keys activate gadget/update face, overlays help/confirm), App state (consumes Session), headless path (print events+roasts+face in voice), TDD Buffer tests for renders/voice/glitch/layout. manic readable. keyboard first.
│   └── ... (more gadget modules, recording, etc in future steps)
└── tests/               # integration tests (real command wrap, etc) -- empty for skeleton
```

annotated:
- src/ mirrors the high-level layers from spec exactly (cli, interceptor, event bus via session, gadget engine, personality, renderer, config)
- gadgets/ for pluggable units behind trait (easy add)
- tui.rs: full impl (widgets + loop + headless + TDD), consumes only (events/state/personality), no mutation
- no over-nesting in v1
- docs/ holds the design spec (do not edit without reason)

## key patterns

- **interception flow**: interceptor (backend pluggable: CommandWrapper or ProxyStub) -> emits Event (CommandOutput, LogLine banner etc) -> session.attach_with_interceptor(vec) or ingest() (the event bus) -> gadgets.apply() + PersonalityHint -> personality + state (distrust, GhostFaceState) -> renderer/headless. strict boundaries (interceptor never sees gadgets; renderer never mutates).
- **gadget pattern**: trait Gadget { name, description (voice), apply(&mut Event) -> Option<PersonalityHint> }. dry_run default true. real mutations explicit + scoped.
- **personality central**: the heart (src/personality.rs). produce_roast(context: &Event, gadget: Option<&str>, state: &GhostFaceState) -> String + update_face_state. all lines (logs, face, reports) through it. voice exact from X/spec: kaomoji mandatory >:[ (¬‿¬) (｡◕‿↼) 👻 💀 XX lmao , blunt "fuck off pete", "zero chill detected 💀", "they ALL talk eventually XX", "digital bully", "recursive gaslighting as a service", "the worst kind of bug...". tests assert exact strings + face transitions (roast -> Party). TDD red-green.
- **data model**: simple copy-friendly enums + structs (Event w/ ts + source(), Session: target/events/active_gadgets + distrust_score/roast_count/mutations + ghost_face_state + dry_run + methods ingest/ingest_event/activate_gadget/get_metrics, PersonalityHint, GhostFaceState enum (Neutral 👻, SideEye (¬‿¬), Roast, Angry >:[ , Party 💀👻(¬‿¬), Skeptical, ZeroChill 💀) + emoji(), SessionMetrics). recordings = vecs.
- **command wrapper (v1 primary)**: std::process::Command exec + capture stdout/stderr to CommandOutput events. always prints + emits "👻 ghost attached (observe only)" banner in voice. dry_run only for banner/observe wording + gadget count (exec of target always happens per attach intent). loud errors on fail ("well that was a silent no-op XX").
- **basic event bus**: in Session (ingest + attach_with_interceptor). updates distrust_score + ghost_face_state on roasts. no channels yet (vec collect for v1 sync attach).
- **safety first**: dry-run everywhere in v1. explicit banners on attach. no auto-mutate. resource limits planned.
- **state management**: Session owns the truth for a run (events vec, counters, active gadgets, dry_run flag). no global.
- **renderer (TUI)**: ratatui widgets custom (face kaomoji from GhostFaceState + intensity, glitch activity on high/party face via spans/invert, bar uses gadget descs+hotkeys in voice, status/livelog from metrics+LogLines), crossterm raw loop + Layout splits per spec (top face 7, main 62/38 activity/gadgets, bottom status+log), overlays (centered popups help/confirm with X voice), keys (q quit, 1-9 gadget, h help, space pause, r roast), resize, App owns Session for consume+activate. headless if !tty || GHOST_HEADLESS (prints events + roasts + face emoji in voice). TDD via Buffer::empty + render asserts on kaomoji/glitch/voice/layout.
- **cli design**: subcommands + trailing args for attach. clap derive. long_about points at spec.
- **testability**: every component has clear inputs/outputs. unit tests in module files. TDD required for gadgets/personality/interceptor paths. full suite 50+ after TUI (incl 7+ new widget/headless/app tests asserting voice/kaomoji).
- **no auth/db**: local only. no external services.

## database schema

none. v1 is in-memory + future file-based session recordings (no sql, no migrations).

## key relationships and migration strategy

- Session owns Events + active Gadgets + PersonalityEngine.
- Gadget implementations live behind dyn Gadget in session.
- PersonalityEngine is stateless in v1 (pure fn on context).
- Interceptor produces Events only; session wires to gadgets/renderer.
- future: add recording persistence (json or bincode of (events, lines)), more gadget files under gadgets/, real interceptor backends.
- no schema migrations. version bumps via cargo + conventional commits.

## environment variables

none in v1. config via toml file or cli flags only. (future: GHOST_DRY_RUN, GHOST_VOICE_LEVEL etc if needed. keep out of code.)

## deployment and infrastructure

- local binary: `cargo build --release` or `cargo install --path .`
- release profile: lto, strip, 1 codegen unit, opt 3 (matches sentinel pattern). small + fast.
- no hosting, no ci/cd in v1, no cron.
- install target: user $PATH (cargo bin). runs on mac/linux (crossterm handles).
- source of truth for structure: this file + design spec. update on every structural change (new modules, deps, etc).

## external services and integrations

none. pure local. (future easy: sentinel hook format compat, gstack health hooks. out of v1 scope.)

## gotchas

- **edition 2024 in cargo.toml**: cargo 1.96 defaulted to it on init. ratatui etc resolved fine. if older rust complains, pin `edition = "2021"`.
- **dry_run is default everywhere**: gadget + session + cli enforce observe-first. if you see real mutation without flag, that's a bug.
- **personality lines must match voice**: if a roast lacks kaomoji or sounds corporate, fix in personality.rs (single source). tests assert on patterns like "(¬‿¬)".
- **ratatui + crossterm versions**: resolved to 0.30/0.29 by cargo (spec listed older). update together if bumping. crossterm backend is default for ratatui.
- **real command wrapper live**: std::process in CommandWrapper.run(dry_run) does actual exec of user command (echo/ls etc tested), emits real captured lines as CommandOutput + banners. old Interceptor::start() kept for compat/skeleton. proxy is stub (no bind).
- **tests use std::time::Instant**: no extra deps for timestamps in v1.
- **clap trailing_var_arg for attach**: allows `ghost attach ./foo --bar` without eating flags. careful with subcommand parsing.
- **TUI + headless/tty**: run uses is_terminal + GHOST_HEADLESS env for decision (no --headless flag on cli in v1 to keep surface minimal). interactive consumes owned Session (for gadget key activates that update face/logs). always clean exit (disable raw, leave alt). clippy/fmt clean. Buffer tests for widgets (no real term needed).

## commands

```bash
# dev
cargo build
cargo run -- --help
cargo run -- attach echo hi --dry-run   # real wrapper: shows 👻 banner + captured events, voice kaomoji, no gadget mut in dry. in tty: full TUI (face flips, glitch activity, gadgets bar 1/2, status ZERO CHILL, log, keys q/1/h etc); else headless voice prints
cargo run -- attach ls / --dry-run
GHOST_HEADLESS=1 cargo run -- attach echo hi --dry-run  # force headless
cargo run -- gadgets

# test (tdd style, run often)
cargo test
cargo test -- --quiet
cargo test tui   # or specific ghost_face_renders_kaomoji_for_state etc

# lint + fmt (clean before commit)
cargo fmt -- --check
cargo clippy -- -D warnings

# release build (the real one)
cargo build --release
# binary at target/release/ghost  (stripped + lto)

# install
cargo install --path .

# docs
# (read the spec first)
cat docs/superpowers/specs/2026-06-14-ghost-design.md
```

no db commands.

## notes

- follow conventional commits on changes: `feat(init): ...`, `docs(architecture): ...`
- update this file on any new files, dirs, deps, patterns, envs.
- all public text (readme, help, code comments with examples) in exact @ThatbV voice.
- yagni: no db, no full proxy tls, no 20 gadgets, no video export in v1.
- TUI update 2026-06-14: full ratatui + TDD + voice everywhere in UI strings + headless. ARCH updated same PR.

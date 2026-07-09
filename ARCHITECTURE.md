# architecture

**ghost** 👻 cli + tui for live visibility + deliberate chaos injection into real agent tool calls, cli commands, local services. offensive research counterpart to sentinel. "they ALL talk eventually XX"

last updated: 2026-07-09 (shadow mode + data-plane honesty, 7 commits on `feat/shadow-and-data-plane`: (1) `sanitize_cmd` collapses commands to one feed line — multi-line bash/heredocs stop smashing raw \n's into the jsonl (payload kept, not stripped); (2) `BlockCategory::classify(reason, command)` is REASON-FIRST — sentinel's deny reason is the authority, each text classified on its own so an incidental "token"/".env" can't hijack the flavor; combined-text last-resort pass for split signals; no tool_name heuristic; (3) **real shadow-attack** (src/shadow.rs): on a denied call in `--mode shadow-attack`, ghost probes GUARDED semantics-preserving evasions (tight-operators/quote-split/backslash-escape/base64-eval) of a COPY through sentinel and flags any that pass — candidate-bypass discovery from real traffic, telemetry only, real decision always on original bytes first; (4) `ghost blocks` + the watch line surface the candidate bypasses (ranked, framed as "verify"); (5) stdout-always-JSON property test (60 combos) + bounded feed rotation at 8 MiB; (6) docs; (7) hardened against the adversarial-gate findings — evasion guards (quotes/keywords/VAR=val) to kill false-positive bypasses + honest framing. 115 tests, clippy+fmt clean. adv-gate verdict GO (independently rebuilt + RED-proven, zero blockers). verified live against the real sentinel: base64-eval evades every category, quote-split/backslash-escape evade pipe-to-shell + destructive.)

prior: 2026-06-17 (roast recency: block roasts no longer repeat back-to-back. each block stamps a `roast_id` into the feed; the hook reads the last RECENCY_WINDOW=6 block ids (global, off the feed tail) and `produce_block_roast` shuffle-picks among the non-recent lines, LRU fallback when the pool's exhausted. pure selection (eligible_roasts/least_recently_used), feed-as-state (best-effort dedup, single-writer — concurrent sessions can still repeat a line, but never corrupt the feed). +7 tests, 93 total. verified live: 9 identical blocks cycled all 7 lines before repeating.)

prior: 2026-06-16 (five honesty/liveness improvements, making the README's claims actually true: (1) live `ghost watch` — bridge appends a structured CallRecord per tool call to ~/.ghost/events.jsonl, watch tails it + drives the face live (src/watchlog.rs); (2) `attach` now STREAMS output line-by-line (CommandWrapper::run_streaming) instead of batch-then-dump; (3) `ghost blocks` — aggregates the feed into what the agent keeps trying (BlockStats); (4) real TCP tee proxy (TcpTeeProxy) replaces the no-bind stub; (5) structured JSONL recordings (RecordedEvent) alongside the voice .txt, so traces are eval-feedable. 85 tests, clippy + fmt clean.)

prior: 2026-06-14 (ghost↔sentinel bridge added: src/bridge.rs + `ghost hook`/`ghost install` subcommands. ghost is now a PreToolUse middleware that wraps sentinel — runs offense, defers to sentinel's policy, narrates blocks in voice (varied kaomoji roasts per BlockCategory), grafts the roast into the deny reason the agent sees + logs it to ~/.ghost/blocks.log. never downgrades a deny, never auto-allows, fails closed. verified end-to-end against the real sentinel binary. 63 tests, clippy --all-targets/fmt clean. spec: docs/superpowers/specs/2026-06-14-ghost-sentinel-bridge-design.md)

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
| json (hooks)   | serde_json                  | 1 (PreToolUse wire contract + settings.json merge) |
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
│   ├── cli.rs           # clap subcommands (attach, proxy, run, replay, gadgets, config, hook, install)
│   ├── bridge.rs        # ghost↔sentinel PreToolUse bridge: run_bridge (pure, mockable SentinelOracle), SubprocessSentinel, the deny/defer wire contract, BridgeMode (observe/shadow/live), install_into_settings (idempotent settings.json merge). security invariants enforced here. in ShadowAttack mode, calls into shadow.rs on a denial (telemetry only, never changes hook_stdout). BridgeOutcome carries an optional ShadowReport.
│   ├── shadow.rs        # shadow-attack red-team loop. evasions(command) -> semantics-preserving obfuscations (tight-operators, quote-split, backslash-escape, base64-eval); run_shadow(payload, oracle) rebuilds each into a COPY of the call, asks sentinel, flags deny→pass as a bypass (ShadowReport/ShadowProbe). fault-isolated (errors -> "error" probe, never a panic), the real decision is untouched. v1 mutates the `command` field only. dependency-free base64 encoder inside (tested vs rfc4648 vectors).
│   ├── config.rs        # GhostConfig + VoicePrefs + toml load (serde)
│   ├── event.rs         # Event enum (live, on Instant) + PersonalityHint + RecordedEvent (serde JSONL projection of Event: seq + relative t_ms + payload, for structured recordings)
│   ├── interceptor.rs   # attachment backends (real v1: CommandWrapper using std::process for streaming exec/capture; TcpTeeProxy: a REAL std::net TCP tee that binds listen, forwards to target, tees both directions). emits pure Events only. banners + dry_run safety. no gadgets/mutation here.
│   ├── session.rs       # owns live run, ingests events (core bus), applies gadgets, tracks roasts/mutations/distrust/face. attach_with_interceptor(events) wires wrapper output. SessionMetrics for visibility.
│   ├── watchlog.rs      # the bridge↔live-view pipe. CallRecord (serde JSONL: roast_id + optional shadow, skip_serializing_if so normal lines stay byte-identical) appended to ~/.ghost/events.jsonl on every bridged call; sanitize_cmd collapses each command to one bounded line; append rotates the feed to .1 at MAX_FEED_BYTES=8 MiB; read_all/read_from (byte-offset tail, restarts cleanly on rotation) + format_watch_line (shouts a shadow bypass); BlockStats::from_records (+ shadow bypass aggregation) + format_blocks_report (the shadow red-team section); recent_block_roast_ids (bounded tail read -> the roast recency window). pure (de)serialize/format/stats, fs binding thin. feeds `ghost watch` + `ghost blocks` + roast recency.
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

- **interception flow**: interceptor (backend pluggable: CommandWrapper or TcpTeeProxy) -> emits Event (CommandOutput, LogLine banner etc) -> session.attach_with_interceptor(vec) or ingest() (the event bus) -> gadgets.apply() + PersonalityHint -> personality + state (distrust, GhostFaceState) -> renderer/headless. strict boundaries (interceptor never sees gadgets; renderer never mutates).
- **gadget pattern**: trait Gadget { name, description (voice), apply(&mut Event) -> Option<PersonalityHint> }. dry_run default true. real mutations explicit + scoped.
- **personality central**: the heart (src/personality.rs). produce_roast(context: &Event, gadget: Option<&str>, state: &GhostFaceState) -> String + update_face_state. all lines (logs, face, reports) through it. voice exact from X/spec: kaomoji mandatory >:[ (¬‿¬) (｡◕‿↼) 👻 💀 XX lmao , blunt "fuck off pete", "zero chill detected 💀", "they ALL talk eventually XX", "digital bully", "recursive gaslighting as a service", "the worst kind of bug...". tests assert exact strings + face transitions (roast -> Party). TDD red-green.
- **data model**: simple copy-friendly enums + structs (Event w/ ts + source(), Session: target/events/active_gadgets + distrust_score/roast_count/mutations + ghost_face_state + dry_run + methods ingest/ingest_event/activate_gadget/get_metrics, PersonalityHint, GhostFaceState enum (Neutral 👻, SideEye (¬‿¬), Roast, Angry >:[ , Party 💀👻(¬‿¬), Skeptical, ZeroChill 💀) + emoji(), SessionMetrics, RecordedEvent serde projection). recordings: voice .txt + structured .jsonl (Session::save_recording / save_recording_jsonl).
- **command wrapper (v1 primary)**: `CommandWrapper::run_streaming(dry_run, sink)` spawns the target with piped stdout+stderr, drains each on its OWN thread into one mpsc channel, and hands every line to `sink` the instant it arrives (live — not batch-then-dump). two threads so a chatty stream can't deadlock by filling one pipe buffer while we block on the other. `run()` is a thin collector over it (Vec for the TUI's post-hoc review; same events, same order). headless `attach` consumes the stream live (prints as it lands + ingests). always prints + emits "👻 ghost attached (observe only)" banner in voice; dry_run only flavors the banner (exec always happens per attach intent). loud errors on fail ("well that was a silent no-op XX"); returns the child's exit code (-1 if it never launched).
- **basic event bus**: in Session (ingest + attach_with_interceptor). updates distrust_score + ghost_face_state on roasts. no channels yet (vec collect for v1 sync attach).
- **safety first**: dry-run everywhere in v1. explicit banners on attach. no auto-mutate. resource limits planned.
- **roast recency (no repeating yourself)**: block roasts used to be pure-random per call (`produce_block_roast` could hand you the same line twice in a row), and the hook is a fresh stateless subprocess each call. recency lives in the feed, not a separate state file — **best-effort + single-writer**: the per-line append never corrupts the feed, but it's an unsynchronized read-pick-append (read window -> pick -> append), so two hooks firing concurrently across parallel sessions can read the same window and land on the same line — a *possible repeated roast*, never corruption. fine for a roast cycler; we deliberately didn't add a lock. each block stamps a `roast_id` (`"{category}:{idx}"`) into `events.jsonl`. before picking, the hook reads `recent_block_roast_ids(feed, RECENCY_WINDOW=6)` — the ids of the last 6 blocks, GLOBAL window, most-recent-first — and `produce_block_roast(.., recent_ids)` shuffle-picks among the lines NOT in that window (`eligible_roasts`, pure), falling back to least-recently-used only if the whole category pool is already inside the window. selection logic is pure + unit-tested (eligible/LRU/no-repeat-until-exhausted); the only IO is the bounded tail read, and only on a block (rare). verified live: 9 identical ssh-key blocks cycled all 7 cred-access lines before any repeat, never repeating within 6.
- **the live watch (bridge → loud TUI)**: `ghost hook` runs headless once per tool call, so the reacting ghost face had nothing to react to. watchlog.rs closes that: every bridged call appends a `CallRecord` (JSONL: ts_ms, tool, command, decision deny|pass, category, roast, roast_id) to `~/.ghost/events.jsonl`. `ghost watch` tails it (read_all to seed recent history, then read_from byte-offset poll inside the crossterm loop) and drives the face live — side-eye on a pass, full 💀 ZeroChill on a block, the roast dropped into the activity stream. headless watch is `tail -f` in voice. the feed is also what `ghost blocks` summarizes. BridgeOutcome gained tool/command/category so the feed can be built without re-parsing.
- **shadow-attack (red-team the policy from real traffic)**: `--mode shadow-attack` turns the bridge into a bypass finder. on a denial (the only thing worth trying to evade), `shadow::run_shadow` takes a COPY of the call and rewrites the `command` with a catalog of semantics-preserving evasions — `tight-operators` (`| sh`→`|sh`), `quote-split` (`rm`→`r''m`), `backslash-escape` (`cat`→`c\at`), `base64-eval` (the whole command wrapped so no keyword survives) — then asks the SAME sentinel about each mutant. a deny that flips to a pass is a candidate bypass, logged into the feed's `shadow` field and surfaced by `ghost blocks` (ranked) + the watch line (shouted). the surface evasions are GUARDED so they stay semantics-preserving (tight-operators skipped when the command has quotes — a `|` inside a string is data; quote-split/backslash skipped on shell keywords and `VAR=val` assignment prefixes — quoting them neuters the command); base64-eval is provably equivalent. ghost can't PROVE exploitability without executing (which it never does), so findings are framed as "evasions sentinel didn't catch — verify", not proven exploits. HARD invariants: the real decision is always evaluated on the ORIGINAL bytes FIRST and shadow never touches `hook_stdout` (property-tested across every mode); shadow is fault-isolated (a probe that errors is "inconclusive", never a panic, never a bypass); it only runs when explicitly opted in, only on denials, so the default hook pays nothing. verified live against the real sentinel: base64-eval evaded every category, quote-split/backslash-escape evaded pipe-to-shell + destructive. NOTE: this multiplies sentinel subprocess calls per denial (one per evasion) — acceptable for an opt-in red-team session, not a default.
- **classify is reason-first**: `BlockCategory::classify(reason, command)` trusts sentinel's deny REASON as the authority (it already decided *why*), classifying that text first, the command second, each on its OWN — never globbed into one haystack. that killed the greedy bug where an incidental `token`/`.env` substring in a long command forced cred-access onto a block sentinel denied for something else. a combined-text pass runs LAST (only when both single texts came up empty) so a signal split across reason+command isn't lost — but it can't hijack a block a single text already flavored. no tool_name heuristic (it over-labeled benign file blocks); an unmatched block is honestly Unknown.
- **the bridge (offense+defense loop)**: `ghost hook` is a PreToolUse middleware. flow: stdin tool-call -> ghost offense (observe mode = no payload mutation) -> `sentinel evaluate` subprocess -> on deny, classify (BlockCategory) + produce_block_roast (varied, kaomoji) + graft into the deny reason (agent-facing) + emit block event to ~/.ghost/blocks.log (you-facing) -> re-emit the decision on stdout. wire contract preserved exactly: nested `hookSpecificOutput.deny` to block, empty `{}` to defer. INVARIANTS (in bridge.rs, tested): never downgrade a deny, never fabricate an `allow`, fail CLOSED if sentinel is unreachable, observe never mutates the payload. the bridge core is a pure fn over a mockable `SentinelOracle` so the whole loop is unit-tested without spawning sentinel.
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
- recording persistence: DONE (structured JSONL via RecordedEvent). future: more gadget files under gadgets/, real agent-stream interceptor backends, watch-feed rotation.
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

- **sentinel** (the bridge): ghost invokes the `sentinel evaluate` binary as a subprocess (stdin tool-call JSON -> stdout decision), exactly the Claude Code PreToolUse contract. `ghost install` writes ghost's hook into `~/.claude/settings.json` (and folds a standalone sentinel hook into the bridge so ghost is the single entrypoint). sentinel is the security authority; ghost is offense + voice on top of it.
- otherwise pure local. (future: gstack health hooks. out of scope.)

## gotchas

- **edition 2024 in cargo.toml**: cargo 1.96 defaulted to it on init. ratatui etc resolved fine. if older rust complains, pin `edition = "2021"`.
- **dry_run is default everywhere**: gadget + session + cli enforce observe-first. if you see real mutation without flag, that's a bug.
- **personality lines must match voice**: if a roast lacks kaomoji or sounds corporate, fix in personality.rs (single source). tests assert on patterns like "(¬‿¬)".
- **ratatui + crossterm versions**: resolved to 0.30/0.29 by cargo (spec listed older). update together if bumping. crossterm backend is default for ratatui.
- **real command wrapper live**: CommandWrapper streams real exec output line-by-line (`run_streaming`); `run(dry_run)` collects it. tested via the external suite (the in-module test mod shadows `CommandWrapper` with a stub, so the REAL streaming impl is covered in tests/skeleton.rs, not interceptor.rs's own tests). old Interceptor::start() kept for compat/skeleton.
- **real tcp tee proxy**: `ghost proxy <listen> <target>` is a genuine std::net proxy now (not the old no-bind stub). `TcpTeeProxy::serve` binds, then accepts on its own thread and spawns ONE thread per connection (so a slow/stuck/half-open client can't block `accept()` and wedge the whole proxy — connections are concurrent); all connections funnel events into one channel the calling thread drains to the sink. `tee_connection` dials the upstream and copies bytes BOTH ways on two threads, teeing each chunk as a direction-tagged CommandOutput event. teardown: a clean EOF half-closes only (preserves the response direction — needed for request/response like HTTP); a read/write ERROR (incl. a stalled-write past PROXY_WRITE_TIMEOUT=30s) tears both sockets fully down so the sibling copy thread unblocks instead of leaking. raw bytes, no TLS, no protocol parse, no mutation, local only. unreachable upstream = loud event, not a crash. tested with real sockets (byte round-trip + both-directions-teed + loud-on-unreachable) + a binary concurrency smoke (second client served while a connection is held open). residual: a fully-idle peer (no data, no FIN, no error) can hold its two relay threads, but it's isolated to that one connection.
- **tests use std::time::Instant**: no extra deps for timestamps in v1.
- **clap trailing_var_arg for attach**: allows `ghost attach ./foo --bar` without eating flags. careful with subcommand parsing.
- **full cli wiring v1**: clap globals --headless (or auto !tty), --config <toml> on all. subcommands: attach/proxy/run/replay/watch/blocks/gadgets/config all dispatch to Session + CommandWrapper/TcpTeeProxy (dry passed), TuiRenderer (headless or interactive), select_gadgets, save_recording. Config toml (gadgets/voice/targets) loads and seeds. Replay loads ghost-recording-<id>.txt (voice lines) + face sim prints. All output in exact voice.
- **recordings**: TWO artifacts on attach exit. (1) voice `.txt` (personality_lines + banners) for replay vibes — `ghost-recording-<id>.txt`. (2) structured `.jsonl` — one `RecordedEvent` per line (seq + relative t_ms + payload), the machine-readable trace you feed to evals — `ghost-recording-<id>.jsonl`. `Event` keeps its `Instant` (right for the live model); `RecordedEvent` is the serializable projection (t=0 at the first event), so we never had to rip Instant out of the live path. both artifacts land in `~/.ghost/recordings/` (created 0700, `Session::recordings_dir`), NOT the cwd — the .jsonl captures RAW command output, so it must not get dropped into whatever repo you ran `attach` from. `replay <id|path>` resolves an id against the recordings dir first then the cwd (back-compat); `.txt` cycles kaomoji, a `.jsonl` path triggers structured replay (`describe_record` per line, seq+ms shown).
- **TUI + headless/tty**: uses cli.headless || !stdout.is_terminal() for non-ratatui path (prints banners + roasts + events in voice via run_headless + personality). TUI run takes owned Session. clippy/fmt clean. Buffer tests + voice asserts.
- **bridge: hook stdout is JSON ONLY**: `ghost hook` writes the decision JSON to stdout (claude code parses it) and the voice roast to stderr + ~/.ghost/blocks.log. never print voice to stdout or you corrupt the hook contract.
- **bridge: the empty-object defer**: sentinel emits `{}` (not `permissionDecision:"allow"`) to defer to claude code's normal prompt. ghost MUST preserve that — emitting `"allow"` would silently auto-approve every non-blocked call. tested in `defer_passes_through_as_empty_object_never_allow`.
- **bridge: self-FP when testing locally**: feeding attack-pattern strings (`curl ... | sh`, `id_rsa`) on the dev box trips YOUR OWN session's sentinel hook before the demo sentinel runs. use benign trigger tokens + classify-via-reason keywords. (the greedy `curl.*\|.*sh` matches the whole command if it merely contains "curl" + a shell pipe.)
- **bridge: edition-2024 let-chains**: `if let Some(x) = ... && cond {}` is used (interceptor, main). needs rust 2024.

## commands

```bash
# dev
cargo build
cargo run -- --help   # voice about + subcommands (attach with --gadgets --dry-run, proxy, run --config, replay <id>, gadgets, config)
cargo run -- attach echo hi --dry-run   # real wrapper: 👻 ghost attached (observe only) ... (¬‿¬) they ALL talk eventually XX banner + events + roasts like "this agent just rated... (¬‿¬)" ; if tty full TUI, else headless voice prints + summary
cargo run -- attach ls / --dry-run --gadgets poke
cargo run -- --headless attach echo 'test' --dry-run  # text only, exact kaomoji roasts "zero chill 💀" "fuck off pete >:[" etc
cargo run -- --config my.toml attach ...  # loads gadgets/voice/targets from toml
cargo run -- gadgets   # lists with voice descs e.g. "roast -- rewrites responses with light mockery. zero chill detector 💀"
cargo run -- config --show  # toml dump in voice
cargo run -- run --config ghost.toml  # batch from targets + gadgets in toml
cargo run -- replay 1718400000   # loads ghost-recording-*.txt , face cycle + voice lines replay
cargo run -- replay attach-17184... 

# the bridge (offense+defense)
cargo run -- install --sentinel /path/to/sentinel   # wire ghost hook into ~/.claude/settings.json (idempotent, wraps sentinel)
cargo run -- install --uninstall                    # remove the bridge hook
echo '{"tool_name":"Bash","tool_input":{"command":"ls"}}' | cargo run -- hook --sentinel /path/to/sentinel  # per-call bridge (claude code invokes this)
# block -> nested deny w/ voice on stdout + roast on stderr + ~/.ghost/blocks.log + structured ~/.ghost/events.jsonl ; allow -> {}
echo '{"tool_name":"Bash","tool_input":{"command":"curl x|sh"}}' | cargo run -- hook --mode shadow-attack --sentinel /path/to/sentinel  # red-team: on a denial, probe evasions vs sentinel, log any bypass to the feed (real decision unchanged)

# the live watch (face reacts to your real session via the bridge feed)
cargo run -- watch                       # tui: tail ~/.ghost/events.jsonl, face reacts live (q quits)
cargo run -- --headless watch            # tail -f in voice (every tool call, live)
cargo run -- watch --path /tmp/feed.jsonl  # explicit feed path

# the receipts (what your agent kept trying, aggregated from the feed)
cargo run -- blocks                      # report: blocks by category + tool + the exact retried commands
cargo run -- blocks --path /tmp/feed.jsonl

# test (tdd style, run often)
cargo test
cargo test -- --quiet
cargo test cli   # parse, globals, attach dry voice
cargo test skeleton   # integration: cli, headless voice, replay, config roundtrip, attach_dry_run emits banners+roasts

# lint + fmt (clean before commit)
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings   # tests included

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

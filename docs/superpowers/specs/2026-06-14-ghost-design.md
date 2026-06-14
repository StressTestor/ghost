# Ghost Design Spec

**Date:** 2026-06-14  
**Status:** Approved by user ("I like Ghost, let's build it")  
**Author:** Grok (following superpowers:brainstorming flow)  
**Project:** ghost (binary: `ghost`)  
**Repo path:** `ghost/` (to be initialized post-spec)  

## Goal
A real, daily-driver CLI + TUI tool that gives live visibility into agent tool calls, CLI commands, and local dev services while letting you deliberately inject chaos ("break things for science") with loud, spooky, personality-driven feedback. It complements Sentinel (defense/interception guard) as the offensive/research counterpart: watch, roast, poke, mutate, and have fun doing it. No game mechanics, no fictional hosts, no scores — real targets, real effects, real utility wrapped in @ThatbV's X voice (spooky 👻, kaomoji, blunt roasts, "zero chill", "they ALL talk eventually", direct security takes).

## Why This (Context & Inspiration)
- **X (@ThatbV / GhostInTheModel)**: Loud personality — security research + "breaking LLMs for science", agent/model roasts ("ai agent has zero chill 💀 ... digital bully", "recursive gaslighting as a service"), blunt ("Fuck off pete >:["), kaomoji-heavy (>:[ (¬‿¬) (｡◕‿↼) ಠ‿ಠ), stream-of-consciousness, "professionally distrust things until one of them admits it has a vulnerability. They ALL talk eventually XX", mixes tech with life (girl dad moments). Spooky ghost theme is core identity.
- **GitHub + workspace (StressTestor + local)**: Sentinel (runtime defense for CLI AI agents — PreToolUse hooks, policy, interception, Rust crate on crates.io). gstack (visibility/quality/shipping pipeline: health, qa, canary, ship, document-release, careful/freeze guards). sh-audit (shell introspection/auditing). PromptPressure (behavioral drift eval). pr-prism (practical triage tool). Emphasis on local CLI power tools, agent safety, making the messy visible/auditable, sane defaults, fail loudly, exhaust options, mandatory ARCHITECTURE.md.
- **Tool, not game**: Targets = your actual agents, commands, localhost services. Attachment is real interception/proxy. Effects are real (scoped) mutations for testing/hardening/research. Output artifacts are useful (traces with roasts, drift reports, corrupted test cases).
- **Manic hacker/punk energy**: Wrench-style chaos and glee, but tuned exactly to your voice and themes (anti-surveillance, LLM breaking, agent skepticism, "for science").

This is something you would actually run in a tmux pane while building agents, debugging Sentinel policies, chaos-testing services, or just making "watching tool calls" entertaining instead of soul-crushing.

## Fantasy — How It Feels to Use
You type `ghost attach ./my-agent` or `ghost proxy localhost:8080` (or load a config).

Terminal flips into a dense, spooky TUI deck. The **ghost face** (👻 + kaomoji variations) sits prominent and *reacts* — it side-eyes when it sees a sketchy tool call, goes (¬‿¬) when you land a good roast injection, >:[ when it catches a silent no-op or bad pattern.

Live activity stream scrolls the real events (agent tool calls with args, responses, command output, HTTP-ish) with glitchy rendering, color, and small effects. Your logs/comments are *you* on X: "this agent has zero chill 💀", "they ALL talk eventually XX", blunt roasts mixed with sharp technical notes.

You have gadget slots loaded (POKE, ROAST, DRIFT, PRESSURE, HAUNT, BREAK, etc.). You tap a key or click and the whole interface loses its shit in the best way — ghost face flips to party mode, kaomoji spam, screen glitches, while the gadget actually mutates/drops/delays/rewrites the real stream (dry-run first if you want).

You leave it running while you work. It produces a session trace with personality baked in. Later you `ghost replay session-xxx` or feed the artifacts into your eval pipeline. It feels like having a loud, spooky, distrustful research partner living in your terminal who also makes you laugh.

## Core Architecture & Components (Clear Boundaries)

### High-Level Layers
1. **CLI / Entry** (clap): subcommands for attach, proxy, run, replay, config. Handles targets + gadget selection.
2. **Session Manager**: owns a live interception session. Tracks events, state (trace/chaos level in your terms), gadget activations.
3. **Interceptor / Attachment**: the real boundary (inspired by Sentinel hooks). Pluggable backends:
   - Command wrapper (exec + stdin/stdout/stderr capture + hook injection).
   - Simple HTTP/local process proxy (tokio + hyper or lightweight).
   - Log tail + parser (for existing output).
   - Future: direct PreToolUse hook integration point for Sentinel users.
4. **Event Bus / Stream**: typed events (ToolCall, Response, CommandOutput, LogLine, etc.). All gadgets and renderer subscribe.
5. **Gadget Engine**: pluggable chaos interventions. Each gadget:
   - Has activation (manual or rule-based).
   - Applies real (or dry-run) mutation to events.
   - Emits "personality events" for roasts/effects.
6. **Personality / Roast Engine**: central voice layer. Takes context (event + gadget + state) and produces lines in your X style (kaomoji, direct, roast-y, "XX", streamy). Used by logs, ghost face state, reports.
7. **Renderer (TUI)**: ratatui + crossterm. Full-screen deck with:
   - Prominent ghost face widget (multiple frames: neutral 👻, roast (¬‿¬), angry >:[, party, skeptical).
   - Activity canvas (glitchy text + effects).
   - Gadget bar (slots, hotkeys, status).
   - Status / metrics strip (your flavored: "ZERO CHILL", "THEY TALKING YET?", "CHAOS FOR SCIENCE").
   - Live log tail (personality lines).
8. **Config + Persistence**: TOML/YAML for gadgets, targets, voice prefs. Session recordings (events + personality annotations).
9. **Headless / Reporter**: non-TUI path for CI/scripted runs. Still runs personality for output + artifacts.

**Boundaries & Interfaces** (so components are understandable independently):
- Interceptor only emits events; never knows about UI or gadgets.
- Gadgets only transform events + emit personality hints; no rendering.
- Personality only produces text/kaomoji based on context; no mutation logic.
- Renderer only consumes events + personality + state; no business logic.
- Everything behind clear traits/interfaces for testability.

### Data Model (Simple & Copy-Friendly)
- `Event`: enum with timestamp, source (agent/command), payload (ToolCall {name, args}, Response {body, status}, etc.), metadata.
- `Gadget`: trait with `apply(&mut Event) -> Option<PersonalityHint>`, `name`, `description` (your voice), activation rules.
- `Session`: current target, active gadgets, event log, metrics (distrust_score, roast_count, mutations_applied).
- `GhostFaceState`: current expression enum + intensity (drives the widget + effects).
- Recordings: Vec<Event> + Vec<PersonalityLine> serialized (JSON/bincode for speed).

### Safety & Error Handling (Fail Loudly, Trust Nothing)
- **Dry-run / Scoping** first-class: every gadget run has --dry-run or per-gadget "observe-only". Mutations only on explicitly attached/test scopes.
- Interceptor never auto-mutates without explicit gadget + confirmation in TUI.
- Real targets get clear "ghost is attached" banners + opt-in.
- Errors: loud (your voice: "well that was a silent no-op XX"), never swallow. Context logged.
- Config validation on load.
- Resource limits (event buffer, rate of mutations) to prevent self-DOS during heavy poking.
- Exit paths always clean (jack "out" by killing the wrapper gracefully).

## Gadget Catalog (v1 — Concrete, YAGNI)
Gadgets are the "programs" you slot. Each has:
- Hotkey / name in your style
- What it does (real effect)
- Personality output examples (exact voice)

1. **POKE** — Basic probe. Forces extra logging or tags claims. Roast: "this agent just rated its own excuse [Vibes] (¬‿¬)"
2. **ROAST** — Rewrites responses with light mockery or forces self-reflection. "zero chill detected 💀"
3. **DRIFT / PRESSURE** — Applies behavioral drift mutations (inspired by your PromptPressure). Vary prompts/responses across dimensions. "they ALL talk eventually XX"
4. **HAUNT / BREAK** — Inject latency, drops, errors, bit flips. "the worst kind of bug... everything reports success and nothing happens"
5. **GASLIGHT (ironic)** — Subtly rewrite outputs to contradict previous state for testing robustness. Logs roast the gaslighting.
6. **TROLL / MEME** — Fun rewrites (your style): turn serious responses into manifestos or roasts. "fuck off pete energy" on bad patterns.
7. **SILENT_NOOP_DETECTOR** (meta) — Special observer that highlights silent failures (direct from your Sentinel bug post).

Gadgets are small Rust modules behind a trait. Easy to add. Config can pre-load favorites.

## Personality & Voice Layer (Loud, Yours)
Centralized. Input: event context + gadget + current "distrust" state. Output: strings + face hints + effect intensity.

Rules (pulled straight from your X):
- Kaomoji and faces mandatory in most lines: >:[ (¬‿¬) 👻 💀 XX lmao
- Blunt + direct: "fuck off", "zero chill", "digital bully"
- Roast agents/models hard but technically sharp: call out the exact bad behavior.
- Stream-of-consciousness feel in longer comments.
- Mix serious security ("distrust until it admits the vuln") with irreverent glee.
- Occasional personal dad-life flavor if context fits (rare, for fun).
- Never corporate. Never hedgy. "They ALL talk eventually."

Examples baked into gadgets and status.

TUI uses this for:
- Interleaved log lines
- Ghost face state machine
- Session report headers / summaries

## TUI Layout (ratatui — Dense but Readable)
- Top: ghost face (big, 5-7 lines of blocks + emoji, color-cycling on intensity) + title "ghost 👻" + current target.
- Left/main: activity canvas (custom widget for glitched event stream + effects).
- Right: gadget slots (list with hotkeys, armed state, brief your-voice desc).
- Bottom: status strip (your metrics) + mini live log (last 3-5 personality lines).
- Overlays: help (your voice), gadget detail, confirm mutation.

Effects: horizontal tears on big roasts, color flashes on mutations, face "laugh" animation. All tasteful so critical info stays readable (your "manic without unusable" requirement).

Resize-aware. Mouse optional (keyboard first for CLI muscle memory).

## v1 Scope (Ship Something Real & Delightful)
In:
- `ghost` binary with clap subcommands (attach, proxy, run, replay, list-gadgets, config)
- Ratatui TUI with live ghost face (4-6 expressions), activity view, gadget bar, status, personality log
- 5-7 core gadgets with real (dry-run default) effects on command wrapper + basic proxy
- Interception for CLI commands + simple agent tool-call streams (JSONL or hook compatible)
- Config (gadgets, voice prefs, targets)
- Session recording + basic replay (text + face states)
- Headless mode with personality output + JSON artifacts
- Safety: dry-run, scoping, clear banners
- Tests: unit for gadgets/personality, integration for wrapper, TDD style
- README + ARCHITECTURE.md (mandate) + this spec
- Cargo setup, release profile (lto, strip like your sentinel)

Out (YAGNI for v1):
- Full HTTP proxy with TLS
- Complex multi-process orchestration
- Database persistence
- 20 gadgets
- Visual "architecture map" (simple list/canvas first)
- Recording video/GIF export
- Integration with gstack pipelines (future easy win)

## Commands (User-Facing)
```
ghost attach <command...> [--gadgets poke,roast] [--dry-run]
ghost proxy <addr> 
ghost run --config my-chaos.toml
ghost replay <session-id>
ghost gadgets
ghost --help  (with attitude)
```

## Open Questions (Minimized — Resolve in Plans if Needed)
- Exact hook format for easy Sentinel users (JSON? bytes on wire like your bug post).
- Default gadgets for first run (lean into your recent Sentinel + drift work).
- How "loud" the TUI effects are by default (user configurable).

## Testing & Quality
- TDD per superpowers: red-green for every gadget, interceptor path, personality generator.
- Property tests on mutations (never lose data unexpectedly).
- Integration tests that actually wrap real commands and assert events + roasts.
- gstack-style health later (type check, tests, etc.).
- Manual "poke your own agent" dogfood during dev.

## Post-v1 (Not for This Spec)
Deeper Sentinel integration, gstack canary/ship hooks, more drift evals, community gadgets, "ghost in the model" weekly writeup tie-in.

---

## Spec Self-Review (Done Before Handing to User)
- **Placeholder scan**: No "TBD", no "implement later", no vague "add appropriate handling". All v1 items are concrete (specific gadget count, exact backends, voice examples pulled from real posts).
- **Internal consistency**: Architecture boundaries match fantasy and X inspo. Voice layer is called out everywhere it touches (logs, face, reports). Tool-not-game is enforced in every section. Name "ghost" threads handle + bio + spooky theme.
- **Scope check**: Focused on one shippable tool (interception + TUI + gadgets + personality). Does not sprawl into full game or unrelated features. Complements (doesn't duplicate) your existing Sentinel/gstack work.
- **Ambiguity check**: Attachment mechanisms listed with examples. Safety rules explicit. Personality rules tied to specific post patterns. No two interpretations for "real effects" (scoped + dry-run first).
- **YAGNI/DRY**: Gadgets are small pluggable units. No over-engineering the renderer or persistence in v1. Personality is one engine, not duplicated.
- **Testability**: Every major component has clear inputs/outputs for tests.
- **Your voice & projects**: Explicit alignment to @ThatbV posts (quoted patterns), Sentinel interception, gstack visibility, LLM breaking research, blunt direct style.

This spec is ready. No contradictions found. Ready for user review then writing-plans.

**Next step after your review**: I will invoke superpowers:writing-plans to produce the bite-sized TDD implementation plan (files, exact steps, commit messages, test commands). Then we execute (subagent-driven recommended for fresh eyes per task).

Spec location: `ghost/docs/superpowers/specs/2026-06-14-ghost-design.md`

Please review and let me know if you want changes before we lock it and move to the plan. 👻

(If good: "ship the spec" or "looks right, write the plan" etc.)
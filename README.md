# ghost 👻

live visibility + deliberate chaos for your agents, cli commands, and localhost services.

real tool. real targets. real effects (scoped + dry-run first). complements your sentinel interception defense as the offensive "breaking llms for science" side.

i built this because watching tool calls in logs is soul-crushing. wanted a loud, spooky research partner in the terminal that roasts what it sees, pokes the sketchy bits, and makes hardening fun instead of tedious. zero chill. they ALL talk eventually XX.

## the fantasy (how it feels)

```
ghost attach ./my-agent --gadgets poke,roast --dry-run
# or
ghost proxy localhost:8080
```

terminal flips to dense spooky tui. the ghost face 👻 reacts live: side-eyes on sketchy calls, (¬‿¬) on good roasts, >:[ on silent no-ops.

live activity scrolls real events (tool calls, responses, command output) with glitch + color. your comments in the stream: "this agent has zero chill 💀", "recursive gaslighting as a service", blunt + sharp.

hit a key and a gadget fires: screen glitches, face goes party mode, while it actually mutates/drops/delays the stream (or just observes).

leaves session traces with personality baked in. replay later. feed to your evals.

feels like having @ThatbV living in your tmux pane, professionally distrustful until something admits the vuln.

## quick start

```bash
# build
cargo build --release

# see the attitude
./target/release/ghost --help

# list gadgets (voice descriptions included)
./target/release/ghost gadgets

# attach a real thing (stub tui for now)
./target/release/ghost attach ./your-agent --dry-run

# or just play
cargo run -- gadgets
```

requires rust 1.96+ (we pin via the toolchain that built it).

## example `ghost --help`

```
ghost 👻
live visibility + deliberate chaos for your agents, commands, localhost. complements sentinel. they ALL talk eventually XX

Usage: ghost <COMMAND>

Commands:
  attach   Attach to a command / agent process (wrapper + capture).
  proxy    Proxy a local addr (http-ish or raw for now). Simple tokio backend.
  run      Run from a full config file (toml). headless or tui depending on flags.
  replay   Replay a previous session recording (text + face states + roasts).
  gadgets  List available gadgets with your voice descriptions + hotkeys.
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

(the long about points at the spec. no corporate fluff.)

## v1 scope (what ships first)

in:
- `ghost` binary, clap subcommands (attach, proxy, run, replay, gadgets, config path)
- ratatui tui with live ghost face (4-6 expressions), activity, gadget bar, status strip, personality log
- 5-7 core gadgets with dry-run real effects on command wrapper + basic proxy (stubs + a couple working in skeleton)
- interception for cli + simple agent streams
- toml config (gadgets, voice, targets)
- session recording + replay (basic)
- headless mode with roasts + json artifacts
- safety: dry run default, banners, scoping
- tests (unit for gadgets/personality, structure tests)
- readme + mandatory ARCHITECTURE.md + design spec
- release profile with lto + strip

out (yagni):
- full tls proxy
- complex orchestration
- db
- 20 gadgets
- video export
- gstack integration (future)

full details + gadget catalog + exact voice examples: `docs/superpowers/specs/2026-06-14-ghost-design.md`

## commands (user)

```
ghost attach <command...> [--gadgets poke,roast] [--dry-run]
ghost proxy <addr>
ghost run --config my-chaos.toml
ghost replay <session-id>
ghost gadgets
ghost --help
```

## personality & voice

everything that talks (logs, face, help text, reports) goes through the central personality engine.

rules pulled from real @ThatbV posts:
- kaomoji everywhere that fits: >:[ (¬‿¬) (｡◕‿↼) 👻 💀 XX lmao
- blunt: "fuck off", "zero chill", "digital bully"
- roast the exact bad behavior, technically sharp
- "they ALL talk eventually"
- mix serious security research with irreverent glee
- never corporate. never hedgy.

see src/personality.rs and the gadgets for the source. change voice? change there.

## safety & philosophy

- dry-run / observe-only is the default and first class. mutations require explicit flag + (later) confirm in tui.
- interceptor never mutates on its own.
- clear banners when attached to real targets.
- fail loudly. context on errors.
- local only. runs on your machine. no cloud, no phoning home.
- sane defaults. config optional.

## development

```bash
cargo test          # tdd: add red test, make green, refactor
cargo clippy -- -D warnings
cargo fmt -- --check
cargo build --release
```

see ARCHITECTURE.md for full commands, gotchas, stack, directory map.

follow conventional commits: `feat(gadgets): implement drift mutation`, `docs(readme): add attach example in voice`.

update ARCHITECTURE.md on any structural change (new modules, deps, etc).

## status

this is the init skeleton commit only. core interception, real gadgets, full tui, replay etc come in follow-on atomic tdd steps.

built because the space needed a loud offensive counterpart to the defensive tooling. for science lmao.

👻

(questions / voice feedback / roast the roasts: same place you roast everything else.)

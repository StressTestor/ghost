# ghost 👻

live visibility + deliberate chaos for your agents, cli commands, and localhost services.

real tool. real targets. real effects (scoped + dry-run first). complements your sentinel interception defense as the offensive "breaking llms for science" side.

i built this because watching tool calls in logs is soul-crushing. wanted a loud, spooky research partner in the terminal that roasts what it sees, pokes the sketchy bits, and makes hardening fun instead of tedious. zero chill. they ALL talk eventually XX.

## the fantasy (how it feels)

```
ghost attach ./my-agent --gadgets poke,roast --dry-run
# or tee a localhost service (real TCP proxy: listen -> target)
ghost proxy 127.0.0.1:8080 127.0.0.1:3000
```

terminal flips to dense spooky tui. the ghost face 👻 reacts: side-eyes on sketchy calls, (¬‿¬) on good roasts, >:[ on silent no-ops.

the face reacting LIVE to a real session is `ghost watch` (it tails the bridge feed). `attach` streams output live in headless; its tui opens on the captured trace once the command's done.

activity scrolls real events (tool calls, responses, command output) with glitch + color. your comments in the stream: "this agent has zero chill 💀", "recursive gaslighting as a service", blunt + sharp.

hit a key and a gadget fires: screen glitches, face goes party mode, while it actually mutates/drops/delays the stream (or just observes).

leaves session traces with personality baked in. replay later. and a structured `.jsonl` trace (one event per line, seq + timing) you can actually feed to your evals.

feels like having @ThatbV living in your tmux pane, professionally distrustful until something admits the vuln.

## quick start

```bash
# build
cargo build --release

# see the attitude
./target/release/ghost --help

# list gadgets (voice descriptions included)
./target/release/ghost gadgets

# attach a thing (headless streams live; the tui opens on the captured trace)
./target/release/ghost --headless attach ./your-agent --dry-run

# or just play
cargo run -- gadgets
```

requires rust 1.96+ (we pin via the toolchain that built it).

## ghost + sentinel: the bridge 👻🛡️

sentinel blocks the dangerous tool calls. ghost makes sure the agent feels bad about it.

`ghost install` puts ghost in front of sentinel as the PreToolUse hook. on every tool call ghost runs its offense, hands the call to sentinel's policy, and when sentinel blocks something the agent tried, ghost roasts it in your voice. the agent reads the roast in the deny reason. you read it in `~/.ghost/blocks.log`.

```bash
# wire it up (idempotent, folds a standalone sentinel hook into the bridge)
ghost install --sentinel /path/to/sentinel

# what claude code runs per tool call (you don't run this by hand):
printf '%s' '{"tool_name":"Bash","tool_input":{"command":"<a blocked thing>"}}' \
  | ghost hook --sentinel /path/to/sentinel
# block -> nested deny with the roast grafted into the reason
#   "...credential vault was reached for. 👻 oh you wanted the secrets. cute. denied (｡◕‿↼) lmao XX"
# allow -> {} (defers to claude code's normal prompt)
```

ghost is offense bolted onto defense, never a way around it:
- never downgrades a sentinel deny. deny is final. >:[
- never fabricates an allow. a non-block defers, untouched.
- fails closed if sentinel is unreachable.
- observe mode (default) never mutates the real payload.

the roasts vary per block category (cred-access, pipe-to-shell, destructive, persistence, exfil) and are loud as hell. uninstall with `ghost install --uninstall`. full design in `docs/superpowers/specs/2026-06-14-ghost-sentinel-bridge-design.md`.

### watch it live

the bridge runs headless, but it also writes a structured feed: every tool call (block or pass) lands in `~/.ghost/events.jsonl`. `ghost watch` tails that feed and reacts in real time.

```bash
ghost watch              # spooky tui. face side-eyes passes, goes full 💀 on blocks
ghost --headless watch   # tail -f for the feed, every call in voice
```

so it's not "read the logs later". the ghost face reacts to your actual session as it happens. blocks drop their roast straight into the activity stream.

want the receipts instead of the live show? `ghost blocks` reads the same feed and tells you what your agent kept reaching for - by category, by tool, and the exact commands it retried (with an "AGAIN??" on the repeats).

```bash
ghost blocks
#   tool calls seen: 142 | blocked by sentinel: 7
#   --- by category ---
#     cred-access: 4 💀
#     pipe-to-shell: 3 💀
#   --- what it kept trying ---
#     3x  <the thing it would not stop doing> (AGAIN??)
```

### shadow mode: red-team sentinel for real

observe mode narrates. shadow mode probes.

when sentinel blocks a call, `--mode shadow-attack` takes a copy of it, disguises the same intent a few ways an attacker would (tighten the pipe, quote-split the command, base64 the whole thing), and asks sentinel about each one. if a disguise gets a pass where the plain call got a deny, that's a bypass. a hole in your defense, found from your own traffic, before anyone else finds it.

```bash
ghost hook --mode shadow-attack --sentinel /path/to/sentinel
# ... later ...
ghost blocks
#   --- shadow red-team (could sentinel be evaded?) ---
#     💀 3 blocked call(s) had an evasion sentinel DIDN'T catch - candidate bypasses, verify:
#       base64-eval: passed 3x - confirm it still does the deed, then patch the policy >:[
```

ghost surfaces candidates, you confirm the deed. it can't prove a disguise still does the same damage without running it (which it never does), so the surface tricks are guarded to stay honest and the loud ones like base64 re-run the exact bytes. treat a finding as "sentinel let this through, go look", not a proven exploit.

the real decision never changes. sentinel still enforces on the original call, always evaluated first. shadow only ever asks questions of a copy, and logs the answers. it costs an extra sentinel call per evasion, so it's opt-in: flip it on when you're deliberately trying to break your own policy, leave it off the rest of the time. observe for the daily driver, shadow for hardening.

## example `ghost --help`

```
ghost 👻
live visibility + deliberate chaos for your agents, commands, localhost. complements sentinel. they ALL talk eventually XX

Usage: ghost [OPTIONS] <COMMAND>

Commands:
  attach   Attach to a command / agent process (wrapper + live capture)
  proxy    Real TCP tee proxy: bind <listen>, forward to <target>, tee both ways
  run      Run from a full config file (toml)
  replay   Replay a previous session recording (.txt voice, or structured .jsonl)
  hook     PreToolUse bridge: run offense, defer to sentinel, narrate the verdict
  install  Wire the ghost↔sentinel bridge into ~/.claude/settings.json
  watch    Tail the bridge feed live and drive the ghost face in real time
  blocks   What your agent kept trying: blocks by category / tool / command
  gadgets  List available gadgets with your voice descriptions + hotkeys
  config   Inspect ghost config (toml)
  help     Print this message or the help of the given subcommand(s)

Options:
      --headless        text only output with full voice. auto if no tty
      --config <CONFIG> path to ghost config toml for gadgets/voice/targets
  -h, --help            Print help
  -V, --version         Print version
```

(the long about points at the spec. no corporate fluff.)

## v1 scope (what ships first)

in:
- `ghost` binary, clap subcommands (attach, proxy, run, replay, gadgets, config path)
- ratatui tui with live ghost face (4-6 expressions), activity, gadget bar, status strip, personality log
- 5-7 core gadgets with dry-run real effects on the command wrapper
- real TCP tee proxy (binds, forwards, tees both directions; no TLS)
- interception for cli + simple agent streams
- toml config (gadgets, voice, targets)
- session recording (voice .txt + structured .jsonl you can feed to evals) + replay
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
ghost proxy <listen> <target>       # real TCP tee proxy: forward listen -> target, watch both ways
ghost run --config my-chaos.toml
ghost replay <session-id>
ghost watch [--path <feed.jsonl>]    # tail the bridge feed live, face reacts in real time
ghost blocks [--path <feed.jsonl>]   # what your agent kept trying: blocks by category/tool/command
ghost gadgets
ghost install --sentinel <path>      # wire the bridge into claude code (wraps sentinel)
ghost hook --sentinel <path>         # the per-call bridge (claude code invokes this)
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

v1 is real: streaming interception, real TCP tee proxy, 7 gadgets, full tui, headless, config, structured + voice recordings, the sentinel bridge (`ghost hook` / `ghost install`) verified end-to-end against the real sentinel binary, and the live `ghost watch` / `ghost blocks` views off the bridge feed. 85 tests, clippy + fmt clean.

built because the space needed a loud offensive counterpart to the defensive tooling. for science lmao.

👻

(questions / voice feedback / roast the roasts: same place you roast everything else.)

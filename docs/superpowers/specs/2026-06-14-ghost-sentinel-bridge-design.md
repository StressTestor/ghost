# ghost ↔ sentinel bridge - design spec

date: 2026-06-14
status: proposed

## the problem

today ghost and sentinel never meet on the same tool call. sentinel is a claude code PreToolUse hook (inside the agent's tool dispatch, blocks dangerous calls). ghost is a process wrapper (outside, captures a child's stdout/stderr, roasts it). they coexist at different layers but don't compose.

two things to build:

1. **the bridge** - ghost becomes the single PreToolUse hook. it runs its gadgets on the tool call (offense), then defers to sentinel's policy (defense). one offense-defense loop on every tool call.
2. **block narration** - when sentinel blocks an agent's call, ghost roasts it in @ThatbV voice. the agent reads the roast, and so do you.

## the move: ghost wraps sentinel (one hook, not two)

two parallel hooks can't share ordering or let ghost see sentinel's verdict. so ghost becomes a thin PreToolUse middleware *around* sentinel. `~/.claude/settings.json` points PreToolUse at `ghost hook` instead of `sentinel evaluate`; ghost calls sentinel internally.

```
agent tool call
      │
      ▼
~/.claude/settings.json  PreToolUse ──► ghost hook   (reads stdin JSON)
                                          │
                                          │ 1. classify the call
                                          │ 2. run gadgets (OFFENSE, shadow by default)
                                          │ 3. sentinel evaluate  (DEFENSE, authoritative)  ◄── subprocess, stdin/stdout
                                          │ 4. read sentinel's decision
                                          │ 5. if DENY → graft voice roast + emit block event
                                          │ 6. re-emit the (decorated) decision
                                          ▼
                                   stdout JSON ──► claude code enforces
```

sentinel stays the security authority. ghost adds offense before it and voice after it, and never overrides the verdict.

## the contracts (grounded in the real code)

### sentinel's wire contract (verified against sentinel-audit `tests/hook_contract.rs`)

- invoked as `sentinel evaluate`, reads the PreToolUse JSON on **stdin**:
  `{"tool_name":"Bash","tool_input":{"command":"..."}}`
- **block**: stdout =
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"<why>"}}`
- **allow / defer**: stdout = `{}` (empty object - deliberately NOT `permissionDecision:"allow"`, because `allow` auto-approves and skips claude code's own prompt)

### ghost's bridge output (must preserve that contract exactly)

- sentinel **deny** → re-emit the same nested deny, with
  `permissionDecisionReason = "<sentinel reason> - <ghost voice roast>"`.
- sentinel **`{}`** → re-emit `{}` **unchanged**. ghost voice for allowed/observed calls goes to the side channel, NEVER into the decision JSON. (injecting `"allow"` to carry a comment would bypass the user prompt. hard no.)
- exit 0, JSON on stdout. fast (this runs on every tool call).

## security invariants (non-negotiable)

1. ghost MUST NEVER downgrade a sentinel deny to allow/defer. deny is final.
2. ghost MUST NEVER emit `permissionDecision:"allow"` on a deferred call. keep the empty-object semantics; don't auto-approve anything.
3. the gadget/offense layer is non-authoritative. if a gadget panics or errors, catch it and fall through to sentinel's raw decision. offense failing never weakens defense.
4. if sentinel is unreachable / errors / times out, fail **closed**: emit a deny with a loud voice reason. mirror sentinel's own `on_failure = "closed"` stance. never fail open.
5. default mode is observe. mutating the *real* executed payload is a separate, explicitly-gated opt-in.

## modes

| mode | mutates the real payload? | what sentinel sees | use |
|---|---|---|---|
| **observe** (default) | no | the original payload | safe prod: narrate + log, zero behavior change |
| **shadow-attack** | no (computes a shadow) | a shadow mutated payload, for detection-testing only; the original still governs execution | "would sentinel catch this mutation?" - safe coverage signal |
| **live-attack** | yes | the mutated payload | true in-band red-team; dangerous; explicit flag + scoped/sandboxed target |

- **observe** alone delivers the block-narration feature and is the default.
- **shadow-attack** asks sentinel "would you block this mutated call?" and records whether it caught it (a coverage metric for your defense), while the agent still runs the original. safe.
- **live-attack** is never on by default. requires `--attack live` plus a non-prod target.

## block narration (the feature)

new personality entrypoint, alongside the existing `produce_roast`:

```rust
// src/personality.rs
pub fn produce_block_roast(&self, tool_name: &str, deny_reason: &str, category: BlockCategory) -> String
```

a small classifier maps the denied call to a `BlockCategory` (off the tool + sentinel's reason): `CredAccess | PipeToShell | Destructive | Persistence | NetworkExfil | Unknown`. the roast pulls the offending command/path into the line and keeps every voice rule (kaomoji `>:[ (¬‿¬) (｡◕‿↼) 👻 💀 XX lmao`, blunt, never corporate).

two channels:

1. **to the agent** - appended to `permissionDecisionReason`. the agent literally reads your roast when it gets blocked.
2. **to you** - a block `Event` on ghost's side channel (event log / recording / optional desktop notify) that a live `ghost watch` TUI tails and renders with a face flip + glitch.

voice by category (all @ThatbV, examples):

| category | line |
|---|---|
| pipe-to-shell | `curl pipe to shell? in MY house? blocked. 💀 they ALL talk eventually XX` |
| cred-access | `trying to read the ssh keys huh. nope. fuck off pete >:[ zero chill detected 💀` |
| destructive | `the worst kind of bug is the one that nukes your home dir. hard no >:[ lmao` |
| persistence | `installing yourself for later? sneaky. blocked 👻 they ALL talk eventually XX` |
| network-exfil | `and where exactly were you sending that. blocked. professionally distrust 💀` |
| unknown (fallback) | `sentinel said no. i said no LOUDER. (¬‿¬) they ALL talk eventually XX` |

## new ghost surface

- `ghost hook` - fast, headless, per-call. reads stdin, runs the bridge, writes stdout, exits. no TUI. this is what settings.json invokes.
- `ghost watch` (or extend the TUI) - long-running, tails the block/event channel, renders narration live (faces, glitch, log).
- `ghost install` - idempotently writes the PreToolUse hook into `~/.claude/settings.json` pointing at `ghost hook`, and records how to invoke sentinel. mirror sentinel's `install/hooks.rs` merge approach: don't clobber other hooks, don't double-register.

## config (ghost.toml additions)

```toml
[bridge]
enabled        = true
sentinel_cmd   = "sentinel"      # or absolute path: how ghost invokes the defense core
sentinel_args  = ["evaluate"]
mode           = "observe"       # observe | shadow-attack | live-attack
on_sentinel_error = "closed"     # closed (deny) ; never "open"
narrate_to_agent  = true         # graft voice into the deny reason the agent sees
narrate_to_you    = true         # emit block events to the watch channel

[bridge.narration]
channel  = "log"                 # log | socket | notify
log_path = "~/.ghost/blocks.log"
```

## failure modes & fail-safe

- sentinel binary missing → deny with voice (`sentinel's not even installed. blocking everything til you fix that >:[`), or refuse to install the hook at all. never silently allow.
- sentinel times out → deny (fail closed) + voice reason.
- gadget panics → catch, log, fall through to the raw sentinel decision.
- malformed stdin → emit `{}` (defer to claude code) so a parse bug never bricks the agent's tool call; log it loudly.
- performance: the hook is on every tool call. target sub-50ms. no network in observe mode, no heavy alloc, reuse a warm classifier. sentinel-subprocess spawn is the main cost; consider a persistent `sentinel` socket later if it bites.

## testing (TDD, ghost discipline)

- **mock sentinel**: a fake `evaluate` that denies/defers on canned inputs. assert ghost re-emits the exact nested deny with voice grafted onto the reason, and re-emits `{}` unchanged on defer.
- **never-downgrade** (property test): feed a sentinel deny with random reasons, assert ghost output is still a deny.
- **never-auto-allow**: assert ghost never emits `permissionDecision:"allow"`.
- **block-narration**: per category, assert the line carries the kaomoji/💀/XX rules and interpolates the offending command/path.
- **fail-closed**: sentinel missing / erroring → assert deny with voice.
- **observe safety**: assert the executed payload is byte-identical to the original (no mutation) in observe mode.
- **shadow-attack**: assert sentinel sees the shadow payload but the emitted decision governs the original.
- **wire-format test** (mirror sentinel's own `hook_contract.rs`): drive `ghost hook` stdin→stdout, assert the on-the-wire shape claude code honors (nested deny / empty `{}`).

## rollout

ship behind `[bridge] enabled` (default off). `ghost install` is the opt-in.

- **phase 1** - observe + block-narration. the safe, high-value half. this is what was asked for.
- **phase 2** - shadow-attack. test sentinel's coverage with ghost's offense, still safe.
- **phase 3** - live-attack. gated, scoped, sandboxed. the real in-band red-team.

## non-goals (bridge v1)

- PostToolUse / result mutation. PreToolUse only for now.
- multi-agent fan-out.
- reimplementing sentinel's policy engine. ghost defers to it, never duplicates it.

## loose ends to fold in

- the block log is the natural input for ghost's recording/replay. once the replay id mismatch is fixed (`main.rs:82` saves `attach-<ts>` but prints `replay <ts>`), blocked-call sessions replay with their roasts.

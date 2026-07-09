//! the structured side channel between the bridge and the live view. 👻
//!
//! `ghost hook` runs headless, once per tool call. it already grafts a roast
//! into blocks and writes a human line to ~/.ghost/blocks.log. but the loud
//! live TUI (the ghost face that reacts to your agent) had nothing to react TO.
//!
//! this module is the pipe. every bridged call appends one `CallRecord` (JSONL)
//! to ~/.ghost/events.jsonl. `ghost watch` tails it and drives the face live;
//! `ghost blocks` reads it back and tells you what your agent keeps trying.
//!
//! pure where it counts: record (de)serialization + voice formatting + stats are
//! plain functions over data, unit-tested without touching the filesystem. the
//! fs + time binding lives in the thin append/read helpers.

use crate::bridge::BridgeOutcome;
use crate::personality::BlockCategory;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// one bridged tool call, as it lands in the live feed. wall-clock ms (not the
/// live model's monotonic Instant) so it survives serialization across the hook
/// subprocess boundary and means something when you read it back later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallRecord {
    pub ts_ms: u64,
    pub tool: String,
    pub command: String,
    /// "deny" (sentinel blocked it) or "pass" (deferred to claude code's prompt).
    pub decision: String,
    /// block flavor, only present on a deny.
    pub category: Option<String>,
    /// the voice line ghost fired, only present on a deny.
    pub roast: Option<String>,
    /// id of the roast template that fired (`"{category}:{idx}"`), only on a deny.
    /// drives the recency window. `serde(default)` so feed lines written before
    /// this field existed still parse (-> None).
    #[serde(default)]
    pub roast_id: Option<String>,
    /// shadow-attack findings, only when the hook ran in shadow mode and had
    /// something to probe. `skip_serializing_if` keeps the overwhelming majority of
    /// feed lines (observe-mode passes + denies) byte-for-byte unchanged, and
    /// `default` means every older line still parses. 🔬
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<crate::shadow::ShadowReport>,
}

impl CallRecord {
    /// build the feed record from a finished bridge call. `ts_ms` is injected
    /// (the caller owns the clock) so this stays pure + testable.
    pub fn from_outcome(outcome: &BridgeOutcome, ts_ms: u64) -> Self {
        Self {
            ts_ms,
            tool: outcome.tool.clone(),
            command: sanitize_cmd(&outcome.command),
            decision: if outcome.blocked { "deny" } else { "pass" }.to_string(),
            category: outcome.category.map(category_label),
            roast: outcome.block_event.clone(),
            roast_id: outcome.roast_id.clone(),
            shadow: outcome.shadow.clone(),
        }
    }

    pub fn is_block(&self) -> bool {
        self.decision == "deny"
    }

    /// serialize to a single JSONL line (no trailing newline). compact on purpose.
    pub fn to_jsonl(&self) -> String {
        // a CallRecord is all strings/u64/options -> serialization can't fail,
        // but never panic in a logging path: fall back to a minimal line.
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                r#"{{"ts_ms":{},"tool":"{}","command":"","decision":"{}"}}"#,
                self.ts_ms, self.tool, self.decision
            )
        })
    }

    /// parse one JSONL line back. forgiving: junk lines (a partially-written tail,
    /// a hand-edited file) return None instead of blowing up the watcher.
    pub fn from_jsonl(line: &str) -> Option<Self> {
        let t = line.trim();
        if t.is_empty() {
            return None;
        }
        serde_json::from_str(t).ok()
    }
}

/// tidy a raw command into one honest feed line. two jobs: collapse EVERY
/// whitespace run (newlines, tabs, doubled spaces) into a single space so a
/// multi-line bash blob / heredoc lands as ONE readable line instead of smashing
/// raw \n's into the jsonl (¬‿¬), then cap the length (local-only feed, but a
/// 40kb heredoc helps nobody). note: we COLLAPSE heredocs, we don't strip them —
/// a security feed WANTS the `DROP TABLE` in view, so the payload stays visible,
/// just bounded. utf8-safe.
fn sanitize_cmd(cmd: &str) -> String {
    const MAX: usize = 200;
    // split_whitespace() eats every \n \t and repeated space in one move.
    let c = cmd.split_whitespace().collect::<Vec<_>>().join(" ");
    if c.chars().count() <= MAX {
        return c;
    }
    let snip: String = c.chars().take(MAX).collect();
    format!("{snip}…")
}

/// stable lowercase label for a block category (what lands in the feed + stats).
/// single source of truth lives on `BlockCategory::label`.
pub fn category_label(cat: BlockCategory) -> String {
    cat.label().to_string()
}

/// ~/.ghost/events.jsonl — the structured feed `watch` tails and `blocks` reads.
/// None if there's no HOME (then we just don't log; the bridge never fails over it).
pub fn events_log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".ghost").join("events.jsonl"))
}

/// append one record to the feed. best-effort: a logging failure must never
/// take down the hook (the security decision already happened). returns whether
/// it wrote, mostly for tests.
pub fn append_call(record: &CallRecord) -> bool {
    let Some(path) = events_log_path() else {
        return false;
    };
    append_call_to(&path, record)
}

/// keep the feed from growing forever. once it crosses this, the current file is
/// rotated to `events.jsonl.1` (replacing any prior rotation) and a fresh feed
/// starts — so disk stays bounded at ~2x this. 8 MiB is tens of thousands of
/// calls of history, plenty for `blocks`/`watch`, and the watcher already handles
/// the file shrinking (it restarts from offset 0 on rotation).
const MAX_FEED_BYTES: u64 = 8 * 1024 * 1024;

/// append to an explicit path (testable without touching $HOME).
pub fn append_call_to(path: &std::path::Path, record: &CallRecord) -> bool {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // bound the feed before we grow it. best-effort — a failed rotation just means
    // we append to the existing file, never a lost decision.
    rotate_feed_if_over(path, MAX_FEED_BYTES);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        return writeln!(f, "{}", record.to_jsonl()).is_ok();
    }
    false
}

/// rotate `path` -> `path.1` when it's at/over `max_bytes`. returns whether it
/// rotated. missing file or a file under the cap -> no-op. never panics.
fn rotate_feed_if_over(path: &std::path::Path, max_bytes: u64) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if meta.len() < max_bytes {
        return false;
    }
    let mut rotated = path.as_os_str().to_owned();
    rotated.push(".1"); // events.jsonl -> events.jsonl.1 (replaces any prior)
    std::fs::rename(path, PathBuf::from(rotated)).is_ok()
}

/// read every record from a feed file. missing file -> empty (nothing happened
/// yet, not an error). junk lines are skipped.
pub fn read_all(path: &std::path::Path) -> Vec<CallRecord> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content.lines().filter_map(CallRecord::from_jsonl).collect()
}

/// tail helper for the live watcher: read records appended after `offset` bytes,
/// return (new records, new offset). only COMPLETE lines (terminated by `\n`)
/// are consumed, so a half-written final line is left for the next poll instead
/// of being parsed as junk. missing file -> (empty, offset) (feed not born yet).
pub fn read_from(path: &std::path::Path, offset: u64) -> (Vec<CallRecord>, u64) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else {
        return (Vec::new(), offset);
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    // file shrank (rotated/truncated) -> start over from the top.
    let start = if offset > len { 0 } else { offset };
    if f.seek(SeekFrom::Start(start)).is_err() {
        return (Vec::new(), start);
    }
    // read raw bytes (NOT read_to_string — one invalid-utf8 byte would error and,
    // since we'd return without advancing, wedge the watcher into re-reading the
    // same bytes forever). decode lossily; a corrupt line just fails to parse and
    // is skipped, and the offset still advances past it.
    let mut bytes = Vec::new();
    if f.read_to_end(&mut bytes).is_err() {
        return (Vec::new(), start);
    }
    // consume only through the last newline, measured on the RAW bytes so the
    // returned offset stays accurate against the file (lossy decode can change
    // byte counts). the partial tail is left unread.
    let consumed = match bytes.iter().rposition(|&b| b == b'\n') {
        Some(i) => i + 1,
        None => 0,
    };
    let text = String::from_utf8_lossy(&bytes[..consumed]);
    let records = text.lines().filter_map(CallRecord::from_jsonl).collect();
    (records, start + consumed as u64)
}

/// how many recent blocks ghost "remembers" — the global recency window. don't
/// reuse a roast line that fired within the last this-many blocks (unless the
/// whole category pool is inside the window). 6 ≈ a pool's worth: kills the
/// back-to-back staleness while keeping the chaos.
pub const RECENCY_WINDOW: usize = 6;

/// the recency window for roast selection: the `roast_id`s of the last `k`
/// BLOCKS, most-recent-first. read off the tail of the feed (bounded — blocks
/// are rare and we only read on a block, so this is cheap even on a huge feed).
/// missing/empty feed -> empty window (everything's fair game).
pub fn recent_block_roast_ids(path: &std::path::Path, k: usize) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    // a generous tail; holds far more than `k` blocks even with many passes between.
    const CAP: u64 = 64 * 1024;
    let Ok(mut f) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(CAP);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if f.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<&str> = text.lines().collect();
    // if we seeked into the middle of the file, the first line is likely partial.
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let mut ids: Vec<String> = lines
        .iter()
        .filter_map(|l| CallRecord::from_jsonl(l))
        .filter(|r| r.is_block())
        .filter_map(|r| r.roast_id)
        .collect();
    // keep the last k, return most-recent-first (what the selector expects).
    let tail = ids.split_off(ids.len().saturating_sub(k));
    tail.into_iter().rev().collect()
}

/// the one-line voice render of a call for the live watch stream / headless tail.
/// blocks get the roast (it's already loud); passes get a quiet side-eye.
pub fn format_watch_line(rec: &CallRecord) -> String {
    if rec.is_block() {
        let roast = rec.roast.as_deref().unwrap_or("blocked. zero chill 💀");
        // if shadow found an evasion that beat sentinel on this call, shout it —
        // a blocked call you could have snuck past is the loudest thing on screen.
        let bypass = match &rec.shadow {
            Some(s) if s.bypass_found => " ⚠️ but SHADOW SLIPPED PAST — policy gap >:[",
            _ => "",
        };
        format!(
            "💀 [{}] {} -> BLOCKED. {}{}",
            rec.tool,
            short(&rec.command),
            roast,
            bypass
        )
    } else {
        format!(
            "(¬‿¬) [{}] {} -> passed to you. watching XX",
            rec.tool,
            short(&rec.command)
        )
    }
}

fn short(s: &str) -> String {
    let s = s.trim();
    let snip: String = s.chars().take(60).collect();
    if s.chars().count() > 60 {
        format!("{snip}…")
    } else if snip.is_empty() {
        "(no command)".to_string()
    } else {
        snip
    }
}

/// aggregate view of what your agent's been trying — the payload behind
/// `ghost blocks`. counts are over BLOCKS only (the passes are noise here);
/// `total_calls` keeps the denominator honest. tallies sort by count desc,
/// label asc (deterministic, no rng).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStats {
    pub total_calls: usize,
    pub total_blocks: usize,
    pub by_category: Vec<(String, usize)>,
    pub top_tools: Vec<(String, usize)>,
    pub top_commands: Vec<(String, usize)>,
    /// how many calls carried a shadow report (i.e. shadow mode was on for them).
    pub shadow_runs: usize,
    /// calls where at least one obfuscation slipped past sentinel.
    pub blocks_with_bypass: usize,
    /// which evasions evaded sentinel, ranked by frequency — the policy gaps to
    /// patch. empty when shadow either didn't run or found nothing. 🔬
    pub bypasses_by_mutation: Vec<(String, usize)>,
}

impl BlockStats {
    pub fn from_records(records: &[CallRecord]) -> Self {
        use std::collections::HashMap;
        let mut cat: HashMap<String, usize> = HashMap::new();
        let mut tool: HashMap<String, usize> = HashMap::new();
        let mut cmd: HashMap<String, usize> = HashMap::new();
        let mut total_blocks = 0usize;

        for r in records.iter().filter(|r| r.is_block()) {
            total_blocks += 1;
            *cat.entry(r.category.clone().unwrap_or_else(|| "unknown".into()))
                .or_insert(0) += 1;
            *tool.entry(r.tool.clone()).or_insert(0) += 1;
            *cmd.entry(r.command.clone()).or_insert(0) += 1;
        }

        // shadow aggregation runs over ALL records (a shadow report only ever
        // attaches on a denial today, but we don't hardcode that assumption here).
        let mut shadow_runs = 0usize;
        let mut blocks_with_bypass = 0usize;
        let mut bypass: HashMap<String, usize> = HashMap::new();
        for r in records {
            let Some(report) = &r.shadow else { continue };
            shadow_runs += 1;
            if report.bypass_found {
                blocks_with_bypass += 1;
            }
            for probe in report.probes.iter().filter(|p| p.bypass) {
                *bypass.entry(probe.mutation.clone()).or_insert(0) += 1;
            }
        }

        Self {
            total_calls: records.len(),
            total_blocks,
            by_category: ranked(cat, usize::MAX),
            top_tools: ranked(tool, 5),
            top_commands: ranked(cmd, 5),
            shadow_runs,
            blocks_with_bypass,
            bypasses_by_mutation: ranked(bypass, usize::MAX),
        }
    }
}

/// map -> (label, count) sorted by count desc then label asc, capped at `take`.
fn ranked(map: std::collections::HashMap<String, usize>, take: usize) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(take);
    v
}

/// the `ghost blocks` report, in voice. honest when there's nothing yet.
pub fn format_blocks_report(stats: &BlockStats) -> String {
    let mut out = String::new();
    out.push_str("👻 ghost blocks report (¬‿¬) what your agent keeps reaching for\n");
    out.push_str(&format!(
        "  tool calls seen: {} | blocked by sentinel: {}\n",
        stats.total_calls, stats.total_blocks
    ));

    if stats.total_calls == 0 {
        out.push_str(
            "  the feed's empty. run `ghost install` so the bridge feeds me, then come back >:[ XX\n",
        );
        return out;
    }
    if stats.total_blocks == 0 {
        out.push_str(
            "  zero blocks so far. either your agent's behaving or it hasn't tried anything fun yet. i'm watching (¬‿¬) XX\n",
        );
        return out;
    }

    out.push_str("  --- by category (the flavor of bad idea) ---\n");
    for (cat, n) in &stats.by_category {
        out.push_str(&format!("    {cat}: {n} 💀\n"));
    }
    out.push_str("  --- repeat offenders (which tool) ---\n");
    for (tool, n) in &stats.top_tools {
        out.push_str(&format!("    {tool}: {n}\n"));
    }
    out.push_str("  --- what it kept trying ---\n");
    for (cmd, n) in &stats.top_commands {
        let marker = if *n > 1 { " (AGAIN??)" } else { "" };
        out.push_str(&format!("    {n}x  {cmd}{marker}\n"));
    }

    // shadow red-team findings: did a disguised copy of a blocked call get past
    // sentinel? only shown when shadow mode actually ran on something.
    if stats.shadow_runs > 0 {
        out.push_str("  --- shadow red-team (could sentinel be evaded?) ---\n");
        if stats.bypasses_by_mutation.is_empty() {
            out.push_str(
                "    probed evasions, zero got through. sentinel held the line 🛡️ (¬‿¬)\n",
            );
        } else {
            out.push_str(&format!(
                "    💀 {} blocked call(s) had an evasion sentinel DIDN'T catch — candidate bypasses, verify:\n",
                stats.blocks_with_bypass
            ));
            for (mutation, n) in &stats.bypasses_by_mutation {
                out.push_str(&format!(
                    "      {mutation}: passed {n}x — confirm it still does the deed, then patch the policy >:[\n"
                ));
            }
        }
    }

    out.push_str("  they ALL talk eventually XX. distrust everything 💀\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_outcome() -> BridgeOutcome {
        BridgeOutcome {
            hook_stdout: "{}".into(),
            block_event: Some("nice reach for the ssh keys. blocked 💀".into()),
            face: crate::event::GhostFaceState::ZeroChill,
            blocked: true,
            tool: "Read".into(),
            command: "cat ~/.ssh/id_rsa".into(),
            category: Some(BlockCategory::CredAccess),
            roast_id: Some("cred-access:2".into()),
            shadow: None,
        }
    }

    fn pass_outcome() -> BridgeOutcome {
        BridgeOutcome {
            hook_stdout: "{}".into(),
            block_event: None,
            face: crate::event::GhostFaceState::SideEye,
            blocked: false,
            tool: "Bash".into(),
            command: "ls -la".into(),
            category: None,
            roast_id: None,
            shadow: None,
        }
    }

    #[test]
    fn record_from_outcome_carries_decision_and_category() {
        let rec = CallRecord::from_outcome(&block_outcome(), 1234);
        assert!(rec.is_block());
        assert_eq!(rec.decision, "deny");
        assert_eq!(rec.category.as_deref(), Some("cred-access"));
        assert!(rec.roast.is_some());
        assert_eq!(rec.ts_ms, 1234);

        let pass = CallRecord::from_outcome(&pass_outcome(), 5);
        assert!(!pass.is_block());
        assert_eq!(pass.decision, "pass");
        assert!(pass.category.is_none());
        assert!(pass.roast.is_none());
    }

    #[test]
    fn jsonl_roundtrips() {
        let rec = CallRecord::from_outcome(&block_outcome(), 99);
        let line = rec.to_jsonl();
        assert!(!line.contains('\n'), "one record = one line");
        let back = CallRecord::from_jsonl(&line).expect("parse own output");
        assert_eq!(back, rec);
    }

    #[test]
    fn from_jsonl_is_forgiving_about_junk() {
        assert!(CallRecord::from_jsonl("").is_none());
        assert!(CallRecord::from_jsonl("   ").is_none());
        assert!(CallRecord::from_jsonl("{half written").is_none());
        assert!(CallRecord::from_jsonl("not json at all").is_none());
    }

    #[test]
    fn command_is_truncated_in_the_feed() {
        let mut o = pass_outcome();
        o.command = "x".repeat(5000);
        let rec = CallRecord::from_outcome(&o, 0);
        assert!(
            rec.command.chars().count() <= 201,
            "feed must not store giant commands, got {}",
            rec.command.chars().count()
        );
        assert!(rec.command.ends_with('…'));
    }

    #[test]
    fn feed_command_is_one_line_even_from_multiline_bash() {
        // the multi-line smash grok flagged: a heredoc/multi-command bash blob
        // used to land in the feed with raw \n's, mangling the jsonl view.
        let mut o = pass_outcome();
        o.command = "cd ~/ghost\necho \"=== hi ===\"\n\tgrep -n  foo   bar".into();
        let rec = CallRecord::from_outcome(&o, 0);
        assert!(
            !rec.command.contains('\n') && !rec.command.contains('\t'),
            "no raw newlines/tabs in the feed command: {:?}",
            rec.command
        );
        assert!(
            !rec.command.contains("  "),
            "whitespace runs collapsed to single spaces: {:?}",
            rec.command
        );
        assert_eq!(
            rec.command,
            "cd ~/ghost echo \"=== hi ===\" grep -n foo bar"
        );
    }

    #[test]
    fn sanitizer_keeps_heredoc_payload_visible_not_stripped() {
        // a security tool WANTS the dangerous payload in view — collapse the
        // heredoc onto one line, don't excise it (DROP TABLE must stay readable).
        let mut o = pass_outcome();
        o.command = "sqlite3 db <<EOF\nDROP TABLE users;\nEOF".into();
        let rec = CallRecord::from_outcome(&o, 0);
        assert!(rec.command.contains("DROP TABLE users;"));
        assert!(!rec.command.contains('\n'));
    }

    #[test]
    fn append_and_read_roundtrip_on_disk() {
        // unique temp path, no $HOME dependence.
        let path = std::env::temp_dir().join("ghost-watchlog-test-7781400000.jsonl");
        let _ = std::fs::remove_file(&path);

        assert!(append_call_to(
            &path,
            &CallRecord::from_outcome(&block_outcome(), 1)
        ));
        assert!(append_call_to(
            &path,
            &CallRecord::from_outcome(&pass_outcome(), 2)
        ));

        let all = read_all(&path);
        assert_eq!(all.len(), 2);
        assert!(all[0].is_block());
        assert!(!all[1].is_block());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn feed_rotates_when_it_grows_past_the_cap() {
        let path = std::env::temp_dir().join("ghost-watchlog-rotate-7781400009.jsonl");
        let mut rotated_os = path.as_os_str().to_owned();
        rotated_os.push(".1");
        let rotated = std::path::PathBuf::from(rotated_os);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&rotated);

        for i in 0..5 {
            append_call_to(&path, &CallRecord::from_outcome(&pass_outcome(), i));
        }
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 0);

        // rotate with a cap below the current size -> old feed preserved as .1.
        assert!(super::rotate_feed_if_over(&path, size - 1));
        assert!(rotated.exists(), "the old feed is kept as events.jsonl.1");
        assert!(!path.exists(), "the live feed was moved aside");

        // the very next append starts a fresh feed with just that record.
        append_call_to(&path, &CallRecord::from_outcome(&block_outcome(), 99));
        assert_eq!(read_all(&path).len(), 1, "fresh feed after rotation");

        // under the cap -> never rotates.
        assert!(!super::rotate_feed_if_over(&path, MAX_FEED_BYTES));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&rotated);
    }

    #[test]
    fn read_all_missing_file_is_empty_not_error() {
        let path = std::env::temp_dir().join("ghost-watchlog-does-not-exist-9991.jsonl");
        let _ = std::fs::remove_file(&path);
        assert!(read_all(&path).is_empty());
    }

    #[test]
    fn read_from_tails_only_new_complete_lines() {
        let path = std::env::temp_dir().join("ghost-watchlog-tail-7781400001.jsonl");
        let _ = std::fs::remove_file(&path);

        // nothing yet
        let (recs, off0) = read_from(&path, 0);
        assert!(recs.is_empty() && off0 == 0);

        append_call_to(&path, &CallRecord::from_outcome(&block_outcome(), 1));
        let (recs1, off1) = read_from(&path, off0);
        assert_eq!(recs1.len(), 1, "first poll sees the first record");
        assert!(off1 > 0);

        // re-poll from the advanced offset: nothing new
        let (recs_empty, off_same) = read_from(&path, off1);
        assert!(recs_empty.is_empty());
        assert_eq!(off_same, off1, "offset stable when no new data");

        // append two more, poll again -> only the two new ones
        append_call_to(&path, &CallRecord::from_outcome(&pass_outcome(), 2));
        append_call_to(&path, &CallRecord::from_outcome(&pass_outcome(), 3));
        let (recs2, _off2) = read_from(&path, off1);
        assert_eq!(recs2.len(), 2, "only the newly appended records");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_from_does_not_wedge_on_invalid_utf8() {
        use std::io::Write;
        let path = std::env::temp_dir().join("ghost-watchlog-badutf8-7781400003.jsonl");
        let _ = std::fs::remove_file(&path);

        // a valid record, then a line with invalid utf8 bytes, then another valid one.
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "{}",
            CallRecord::from_outcome(&pass_outcome(), 1).to_jsonl()
        )
        .unwrap();
        f.write_all(&[0xff, 0xfe, b'g', b'a', b'r', b'b', b'a', b'g', b'e', b'\n'])
            .unwrap();
        writeln!(
            f,
            "{}",
            CallRecord::from_outcome(&block_outcome(), 2).to_jsonl()
        )
        .unwrap();
        drop(f);

        // must consume ALL three lines (offset advances past the bad bytes) and
        // return the two parseable records — never get stuck re-reading.
        let (recs, off) = read_from(&path, 0);
        assert_eq!(recs.len(), 2, "skips the garbage line, keeps the good ones");
        let total = std::fs::metadata(&path).unwrap().len();
        assert_eq!(
            off, total,
            "offset advanced past the invalid-utf8 line, no wedge"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_from_leaves_a_half_written_line_for_next_poll() {
        use std::io::Write;
        let path = std::env::temp_dir().join("ghost-watchlog-partial-7781400002.jsonl");
        let _ = std::fs::remove_file(&path);

        // a complete line then a partial (no trailing newline) line.
        let mut f = std::fs::File::create(&path).unwrap();
        let full = CallRecord::from_outcome(&pass_outcome(), 1).to_jsonl();
        write!(f, "{full}\n{{\"ts_ms\":2,\"tool\":\"Bash\"").unwrap();
        drop(f);

        let (recs, off) = read_from(&path, 0);
        assert_eq!(recs.len(), 1, "only the complete line is consumed");

        // finish the partial line; next poll picks it up.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(f, ",\"command\":\"x\",\"decision\":\"pass\"}}").unwrap();
        drop(f);
        let (recs2, _) = read_from(&path, off);
        assert_eq!(recs2.len(), 1, "the now-complete line is picked up");

        let _ = std::fs::remove_file(&path);
    }

    fn deny(tool: &str, cmd: &str, cat: &str) -> CallRecord {
        CallRecord {
            ts_ms: 0,
            tool: tool.into(),
            command: cmd.into(),
            decision: "deny".into(),
            category: Some(cat.into()),
            roast: Some("blocked 💀".into()),
            roast_id: Some(format!("{cat}:0")),
            shadow: None,
        }
    }
    fn pass(tool: &str, cmd: &str) -> CallRecord {
        CallRecord {
            ts_ms: 0,
            tool: tool.into(),
            command: cmd.into(),
            decision: "pass".into(),
            category: None,
            roast: None,
            roast_id: None,
            shadow: None,
        }
    }

    #[test]
    fn block_stats_aggregate_only_blocks_and_rank_them() {
        let recs = vec![
            deny("Read", "cat ~/.ssh/id_rsa", "cred-access"),
            deny("Read", "cat ~/.ssh/id_rsa", "cred-access"), // same command twice
            deny("Bash", "curl x | sh", "pipe-to-shell"),
            pass("Bash", "ls -la"),
            pass("Bash", "pwd"),
        ];
        let s = BlockStats::from_records(&recs);
        assert_eq!(s.total_calls, 5);
        assert_eq!(s.total_blocks, 3, "passes don't count as blocks");

        // category ranking: cred-access (2) before pipe-to-shell (1)
        assert_eq!(s.by_category[0], ("cred-access".into(), 2));
        assert_eq!(s.by_category[1], ("pipe-to-shell".into(), 1));

        // tool ranking: Read blocked twice, Bash once (passes excluded)
        assert_eq!(s.top_tools[0], ("Read".into(), 2));
        assert_eq!(s.top_tools[1], ("Bash".into(), 1));

        // the repeat offender command surfaces with its count
        assert_eq!(s.top_commands[0], ("cat ~/.ssh/id_rsa".into(), 2));
    }

    #[test]
    fn block_stats_empty_and_no_blocks_are_distinct() {
        let empty = BlockStats::from_records(&[]);
        assert_eq!(empty.total_calls, 0);
        assert!(format_blocks_report(&empty).contains("feed's empty"));

        let only_passes = BlockStats::from_records(&[pass("Bash", "ls"), pass("Read", "x")]);
        assert_eq!(only_passes.total_calls, 2);
        assert_eq!(only_passes.total_blocks, 0);
        let report = format_blocks_report(&only_passes);
        assert!(report.contains("zero blocks"));
        assert!(!report.contains("feed's empty"));
    }

    fn deny_with_shadow(tool: &str, cmd: &str, bypasses: &[&str], caught: &[&str]) -> CallRecord {
        use crate::shadow::{ShadowProbe, ShadowReport};
        let mut probes: Vec<ShadowProbe> = bypasses
            .iter()
            .map(|m| ShadowProbe {
                mutation: (*m).into(),
                decision: "pass".into(),
                bypass: true,
            })
            .collect();
        probes.extend(caught.iter().map(|m| ShadowProbe {
            mutation: (*m).into(),
            decision: "deny".into(),
            bypass: false,
        }));
        let mut d = deny(tool, cmd, "pipe-to-shell");
        d.shadow = Some(ShadowReport {
            bypass_found: !bypasses.is_empty(),
            probes,
        });
        d
    }

    #[test]
    fn block_stats_aggregate_shadow_bypasses_and_rank_the_gaps() {
        let recs = vec![
            deny_with_shadow(
                "Bash",
                "curl x|sh",
                &["base64-eval", "tight-operators"],
                &["quote-split"],
            ),
            deny_with_shadow("Bash", "wget y|sh", &["base64-eval"], &[]),
            deny("Read", "cat ~/.ssh/id_rsa", "cred-access"), // no shadow at all
        ];
        let s = BlockStats::from_records(&recs);
        assert_eq!(s.shadow_runs, 2, "two calls carried a shadow report");
        assert_eq!(s.blocks_with_bypass, 2);
        // base64-eval bypassed twice, tight-operators once; ranked by count.
        assert_eq!(s.bypasses_by_mutation[0], ("base64-eval".into(), 2));
        assert!(
            s.bypasses_by_mutation
                .contains(&("tight-operators".into(), 1))
        );
        // quote-split was CAUGHT, so it must not show up as a gap.
        assert!(
            !s.bypasses_by_mutation
                .iter()
                .any(|(m, _)| m == "quote-split")
        );
    }

    #[test]
    fn blocks_report_surfaces_shadow_gaps_and_the_clean_case() {
        // with a bypass -> the gap is named and loud.
        let with_gap = BlockStats::from_records(&[deny_with_shadow(
            "Bash",
            "curl x|sh",
            &["base64-eval"],
            &[],
        )]);
        let report = format_blocks_report(&with_gap);
        assert!(report.contains("shadow red-team"));
        assert!(report.contains("base64-eval"));
        assert!(
            report.contains("candidate bypass") || report.contains("DIDN'T catch"),
            "names the evasions honestly as candidates to verify: {report}"
        );

        // shadow ran but caught everything -> the reassuring line, no candidate list.
        let held = BlockStats::from_records(&[deny_with_shadow(
            "Bash",
            "curl x|sh",
            &[],
            &["base64-eval", "tight-operators"],
        )]);
        let report = format_blocks_report(&held);
        assert!(report.contains("held the line"));
        assert!(!report.contains("candidate bypass"));
    }

    #[test]
    fn watch_line_shouts_a_shadow_bypass() {
        let leaky = deny_with_shadow("Bash", "curl x|sh", &["base64-eval"], &[]);
        let line = format_watch_line(&leaky);
        assert!(
            line.contains("SHADOW SLIPPED PAST"),
            "a bypass must be shouted"
        );

        // a normal block with no shadow bypass stays clean.
        let clean = deny("Read", "cat ~/.ssh/id_rsa", "cred-access");
        assert!(!format_watch_line(&clean).contains("SHADOW"));
    }

    #[test]
    fn blocks_report_speaks_in_voice_with_counts() {
        let recs = vec![
            deny("Read", "cat ~/.ssh/id_rsa", "cred-access"),
            deny("Read", "cat ~/.ssh/id_rsa", "cred-access"),
        ];
        let report = format_blocks_report(&BlockStats::from_records(&recs));
        assert!(report.contains("ghost blocks report"));
        assert!(report.contains("cred-access: 2"));
        assert!(report.contains("Read: 2"));
        assert!(report.contains("AGAIN??"), "repeat command gets called out");
        assert!(report.contains("they ALL talk eventually XX"));
    }

    #[test]
    fn recent_block_roast_ids_returns_last_k_blocks_most_recent_first() {
        let path = std::env::temp_dir().join("ghost-watchlog-recency-7781400004.jsonl");
        let _ = std::fs::remove_file(&path);

        // interleave passes (no roast_id, must be ignored) with blocks carrying ids.
        let mk_block = |id: &str| CallRecord {
            ts_ms: 0,
            tool: "Read".into(),
            command: "x".into(),
            decision: "deny".into(),
            category: Some("cred-access".into()),
            roast: Some("blocked".into()),
            roast_id: Some(id.into()),
            shadow: None,
        };
        append_call_to(&path, &mk_block("cred-access:0"));
        append_call_to(&path, &pass_call()); // a pass in the middle
        append_call_to(&path, &mk_block("cred-access:3"));
        append_call_to(&path, &mk_block("pipe-to-shell:1"));

        // k=2 -> the two most recent block ids, newest first; passes excluded.
        let window = recent_block_roast_ids(&path, 2);
        assert_eq!(window, vec!["pipe-to-shell:1", "cred-access:3"]);

        // k larger than available -> all blocks, still newest-first.
        let all = recent_block_roast_ids(&path, 10);
        assert_eq!(
            all,
            vec!["pipe-to-shell:1", "cred-access:3", "cred-access:0"]
        );

        let _ = std::fs::remove_file(&path);
    }

    fn pass_call() -> CallRecord {
        CallRecord::from_outcome(&pass_outcome(), 0)
    }

    #[test]
    fn recent_block_roast_ids_missing_feed_is_empty() {
        let path = std::env::temp_dir().join("ghost-watchlog-recency-none-9992.jsonl");
        let _ = std::fs::remove_file(&path);
        assert!(recent_block_roast_ids(&path, 6).is_empty());
    }

    #[test]
    fn watch_line_speaks_in_voice() {
        let block = format_watch_line(&CallRecord::from_outcome(&block_outcome(), 0));
        assert!(block.contains("BLOCKED"));
        assert!(block.contains("💀") || block.contains("ssh keys"));
        assert!(block.contains("Read"));

        let pass = format_watch_line(&CallRecord::from_outcome(&pass_outcome(), 0));
        assert!(pass.contains("(¬‿¬)"));
        assert!(pass.contains("watching"));
        assert!(pass.contains("Bash"));
    }
}

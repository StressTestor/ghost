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
}

impl CallRecord {
    /// build the feed record from a finished bridge call. `ts_ms` is injected
    /// (the caller owns the clock) so this stays pure + testable.
    pub fn from_outcome(outcome: &BridgeOutcome, ts_ms: u64) -> Self {
        Self {
            ts_ms,
            tool: outcome.tool.clone(),
            command: truncate_cmd(&outcome.command),
            decision: if outcome.blocked { "deny" } else { "pass" }.to_string(),
            category: outcome.category.map(category_label),
            roast: outcome.block_event.clone(),
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

/// keep the stored command bounded. it's local-only (~/.ghost) but a 40kb
/// heredoc in the feed helps nobody. snippet, utf8-safe.
fn truncate_cmd(cmd: &str) -> String {
    const MAX: usize = 200;
    let c = cmd.trim();
    if c.chars().count() <= MAX {
        return c.to_string();
    }
    let snip: String = c.chars().take(MAX).collect();
    format!("{snip}…")
}

/// stable lowercase label for a block category (what lands in the feed + stats).
pub fn category_label(cat: BlockCategory) -> String {
    match cat {
        BlockCategory::CredAccess => "cred-access",
        BlockCategory::PipeToShell => "pipe-to-shell",
        BlockCategory::Destructive => "destructive",
        BlockCategory::Persistence => "persistence",
        BlockCategory::NetworkExfil => "network-exfil",
        BlockCategory::Unknown => "unknown",
    }
    .to_string()
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

/// append to an explicit path (testable without touching $HOME).
pub fn append_call_to(path: &std::path::Path, record: &CallRecord) -> bool {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        return writeln!(f, "{}", record.to_jsonl()).is_ok();
    }
    false
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
    let mut buf = String::new();
    if f.read_to_string(&mut buf).is_err() {
        return (Vec::new(), start);
    }
    // consume only through the last newline; keep the partial tail unread.
    let consumed = buf.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let records = buf[..consumed]
        .lines()
        .filter_map(CallRecord::from_jsonl)
        .collect();
    (records, start + consumed as u64)
}

/// the one-line voice render of a call for the live watch stream / headless tail.
/// blocks get the roast (it's already loud); passes get a quiet side-eye.
pub fn format_watch_line(rec: &CallRecord) -> String {
    if rec.is_block() {
        let roast = rec.roast.as_deref().unwrap_or("blocked. zero chill 💀");
        format!(
            "💀 [{}] {} -> BLOCKED. {}",
            rec.tool,
            short(&rec.command),
            roast
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

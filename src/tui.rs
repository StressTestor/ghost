use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap},
};

use crate::event::{Event, GhostFaceState, RecordedEvent};
use crate::gadgets::default_gadgets;
use crate::session::Session;

/// Renderer (TUI) layer using ratatui + crossterm.
///
/// v1 layout (dense but readable, per spec):
/// - Top: ghost face (big, emoji + kaomoji blocks) + title "ghost 👻" + current target
/// - Main: activity canvas (glitchy event stream + effects)
/// - Right: gadget bar (slots, hotkeys, voice descs)
/// - Bottom: status strip ("ZERO CHILL", "THEY TALKING YET?", "CHAOS FOR SCIENCE") + mini live log
/// - Overlays for help (your voice), confirm mutation
///
/// Effects: color flashes on roasts, face state machine (neutral 👻, (¬‿¬), >:[ , party)
///
/// Boundaries:
/// - Renderer ONLY consumes events + personality + state; NO business logic, NO mutation.
/// - Keyboard first. Mouse optional.
/// - Resize aware.
///
/// Headless: when --headless or !isatty, bypass ratatui entirely. print banners + every roast/event in exact @ThatbV voice kaomoji, summary. still personality driven.
/// Used by main dispatch + replay.
pub struct TuiRenderer {
    // terminal state etc later
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// Run the full TUI loop. Blocks until exit.
    /// Takes owned Session for interactive gadget activation (which delegates to session's personality + face updates).
    /// Real: crossterm raw mode + ratatui draw loop + key poll. Keyboard first per spec.
    pub fn run(&self, session: Session) {
        if let Err(e) = self.run_interactive(session) {
            eprintln!(
                "👻 tui error (well that was a silent no-op XX): {} >:[ they ALL talk eventually",
                e
            );
        }
    }

    #[allow(
        clippy::collapsible_if,
        clippy::unnecessary_to_owned,
        deprecated,
        unused_variables,
        unreachable_patterns
    )]
    fn run_interactive(&self, session: Session) -> io::Result<()> {
        let mut app = App::new(session);

        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(out);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| draw_ui(f, &app))?;

            if event::poll(Duration::from_millis(120))?
                && let CEvent::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('h') => app.show_help = !app.show_help,
                    KeyCode::Char(' ') => app.paused = !app.paused,
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if !app.paused {
                            let idx = (c.to_digit(10).unwrap_or(0) as usize).saturating_sub(1);
                            app.activate_gadget(idx);
                        }
                    }
                    KeyCode::Char('r') => {
                        if !app.paused {
                            app.activate_gadget(1);
                        }
                    }
                    KeyCode::Up => app.scroll = app.scroll.saturating_sub(1),
                    KeyCode::Down => app.scroll = app.scroll.saturating_add(1),
                    _ => {}
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        println!("👻 ghost detached (observe only). (¬‿¬) they ALL talk eventually XX");
        Ok(())
    }

    /// LIVE watch: tail the bridge feed (~/.ghost/events.jsonl) and drive the
    /// ghost face in real time off your actual agent's tool calls. this is the
    /// view the README always promised — the face reacting to a real session.
    /// blocks until `q`/esc. seeds with recent history so it isn't empty.
    pub fn run_watch(&self, path: std::path::PathBuf) {
        if let Err(e) = self.run_watch_interactive(path) {
            eprintln!("👻 watch error (silent no-op XX): {e} >:[ they ALL talk eventually");
        }
    }

    fn run_watch_interactive(&self, path: std::path::PathBuf) -> io::Result<()> {
        let session = Session::new("claude code 👻 live (via sentinel bridge)");
        let mut app = App::new(session);
        app.log_lines.clear();
        app.log_lines.push(
            "👻 watching the bridge feed. waiting for tool calls (¬‿¬) they ALL talk eventually XX"
                .to_string(),
        );

        // seed with recent history AND capture the offset from the same read, so
        // a record appended between "read history" and "start tailing" can't slip
        // through the gap (it'd be in neither). offset = end of what we consumed.
        let (history, mut offset) = crate::watchlog::read_from(&path, 0);
        for rec in history.iter().rev().take(30).rev() {
            app.ingest_call(rec);
        }

        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(out);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| draw_ui(f, &app))?;

            if event::poll(Duration::from_millis(150))?
                && let CEvent::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('h') => app.show_help = !app.show_help,
                    KeyCode::Char(' ') => app.paused = !app.paused,
                    KeyCode::Up => app.scroll = app.scroll.saturating_sub(1),
                    KeyCode::Down => app.scroll = app.scroll.saturating_add(1),
                    _ => {}
                }
            }

            if !app.paused {
                let (new, new_offset) = crate::watchlog::read_from(&path, offset);
                offset = new_offset;
                for rec in &new {
                    app.ingest_call(rec);
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        println!("👻 ghost detached from the live feed. (¬‿¬) they ALL talk eventually XX");
        Ok(())
    }

    /// headless watch: `tail -f` for the bridge feed, every call in voice.
    /// blocks (ctrl-c to stop), so it's the no-tty / piped path.
    pub fn run_watch_headless(&self, path: std::path::PathBuf) {
        println!(
            "👻 ghost watching the bridge feed at {} (headless). every tool call, live. zero chill 💀",
            path.display()
        );
        // seed + capture offset from one read (no gap between history + tail).
        let (history, mut offset) = crate::watchlog::read_from(&path, 0);
        for rec in history.iter().rev().take(30).rev() {
            println!("  {}", crate::watchlog::format_watch_line(rec));
        }
        loop {
            let (new, new_offset) = crate::watchlog::read_from(&path, offset);
            offset = new_offset;
            for rec in &new {
                println!("  {}", crate::watchlog::format_watch_line(rec));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// Headless runner: print events + roasts + face in voice. No ratatui. For --headless / env / !tty.
    pub fn run_headless(&self, session: &Session) {
        println!(
            "👻 ghost headless (no tty or flag). raw event stream + personality roasts. zero chill detected 💀"
        );
        println!("target: {} (they ALL talk eventually XX)", session.target);
        for ev in &session.events {
            println!("  {}", render_event_line(ev));
        }
        let m = session.get_metrics();
        println!("{}", session.summary());
        println!(
            "ghost face: {} | distrust: {} | roasts: {} (｡◕‿↼) CHAOS FOR SCIENCE",
            m.face.emoji(),
            m.distrust_score,
            m.roast_count
        );
        println!("-- end trace -- they ALL talk eventually XX lmao");
    }

    /// Headless / reporter path (no TUI). Still runs personality for artifacts.
    pub fn headless_summary(&self, session: &Session) -> String {
        format!(
            "ghost 👻 headless run complete\n{}\nroasts fired: {}\n-- end trace --",
            session.summary(),
            session.roast_count
        )
    }

    /// basic replay from recording file (produced by Session::save_recording).
    /// loads voice lines (banners + roasts), replays with cycling ghost face sim + prints (for dogfood + headless).
    /// id: "123456" -> ghost-recording-123456.txt or explicit path.
    /// resolve a recording id to a file path. forgiving: an explicit path/.txt/
    /// .jsonl is used as-is; otherwise we search for a `ghost-recording-<id>` file
    /// (txt or jsonl, with the `attach-` prefix variants) first in `~/.ghost/
    /// recordings` (where they live now), then the cwd (back-compat with older
    /// recordings). a bare timestamp still finds its recording.
    fn resolve_recording(id: &str) -> String {
        if id.contains('/') || id.ends_with(".txt") || id.ends_with(".jsonl") {
            return id.to_string();
        }
        let names = [
            format!("ghost-recording-{id}.txt"),
            format!("ghost-recording-{id}.jsonl"),
            format!("ghost-recording-attach-{id}.txt"),
            format!("ghost-recording-attach-{id}.jsonl"),
        ];
        let bases = [
            crate::session::recordings_dir(),
            std::path::PathBuf::from("."),
        ];
        for base in &bases {
            for name in &names {
                let p = base.join(name);
                if p.exists() {
                    return p.display().to_string();
                }
            }
        }
        // nothing found: the canonical name in the recordings dir (replay will
        // report a clean "no recording" miss).
        crate::session::recordings_dir()
            .join(format!("ghost-recording-{id}.txt"))
            .display()
            .to_string()
    }

    /// structured replay: a .jsonl recording (RecordedEvent per line) rendered
    /// back in voice with its relative timing + sequence. this is the trace you
    /// can also hand to evals — replay just makes it human here.
    fn replay_jsonl(path: &str, id: &str) -> String {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let mut out = format!(
                    "👻 replaying structured trace {} (¬‿¬) machine-readable. they ALL talk eventually XX\n",
                    id
                );
                let faces = ["👻", "(¬‿¬)", "💀👻(¬‿¬)", ">:[", "(｡◕‿↼)", "ಠ‿ಠ", "💀"];
                let mut n = 0usize;
                for line in content.lines() {
                    if let Some(rec) = RecordedEvent::from_jsonl(line) {
                        let f = faces[n % faces.len()];
                        let rendered = format!("{} {}", f, describe_record(&rec));
                        println!("{}", rendered);
                        out.push_str(&rendered);
                        out.push('\n');
                        n += 1;
                    }
                }
                let closer = format!(
                    "replay complete. {n} structured events. zero chill 💀 they ALL talk eventually XX"
                );
                println!("{}", closer);
                out.push_str(&closer);
                out.push('\n');
                out
            }
            Err(e) => {
                let msg = format!(
                    "👻 no structured recording for {} at {}. silent no-op XX. err: {}",
                    id, path, e
                );
                println!("{}", msg);
                msg
            }
        }
    }

    pub fn replay(id: &str) -> String {
        let path = Self::resolve_recording(id);
        if path.ends_with(".jsonl") {
            return Self::replay_jsonl(&path, id);
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let mut out = format!(
                    "👻 replaying session {} (¬‿¬) they ALL talk eventually XX\n",
                    id
                );
                let faces = ["👻", "(¬‿¬)", "💀👻(¬‿¬)", ">:[", "(｡◕‿↼)", "ಠ‿ಠ", "💀"];
                for (i, line) in content.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let f = faces[i % faces.len()];
                    let replay_line = format!("{} [replay {}] {}", f, i, line);
                    println!("{}", replay_line);
                    out.push_str(&replay_line);
                    out.push('\n');
                }
                let closer = "replay complete. zero chill detected 💀 they ALL talk eventually XX";
                println!("{}", closer);
                out.push_str(closer);
                out.push('\n');
                out
            }
            Err(e) => {
                let msg = format!(
                    "👻 no recording for {} at {}. well that was a silent no-op XX. run attach first. err: {}",
                    id, path, e
                );
                println!("{}", msg);
                msg
            }
        }
    }
}

/// one event as a voice-flavored line for the headless / live stream. shared by
/// `run_headless` and the live `attach` headless path so batch and streaming
/// output read identically. pure -> unit-tested.
pub fn render_event_line(ev: &Event) -> String {
    match ev {
        Event::LogLine { msg, source, .. } => {
            if source.starts_with("gadget:")
                || msg.contains("👻")
                || msg.contains("zero chill")
                || msg.contains("they ALL")
                || msg.contains(">:[")
                || msg.contains("(¬")
            {
                msg.clone()
            } else {
                format!("[log:{source}] {msg}")
            }
        }
        Event::CommandOutput { line, stream, .. } => format!("[{stream}] {line}"),
        Event::ToolCall { name, args, .. } => format!("[toolcall] {name} args={args}"),
        Event::Response { body, status, .. } => format!("[response] {body} status={status:?}"),
    }
}

/// one-line human description of a structured recording event (for jsonl replay).
/// keeps the [seq] + relative ms so the trace reads in order with timing.
pub fn describe_record(rec: &RecordedEvent) -> String {
    match rec {
        RecordedEvent::ToolCall {
            seq, t_ms, name, ..
        } => format!("[{seq}|{t_ms}ms] tool {name}"),
        RecordedEvent::Response {
            seq, t_ms, status, ..
        } => format!("[{seq}|{t_ms}ms] response status={status:?}"),
        RecordedEvent::CommandOutput {
            seq,
            t_ms,
            line,
            stream,
        } => format!("[{seq}|{t_ms}ms] {stream}: {line}"),
        RecordedEvent::Log { seq, t_ms, msg, .. } => format!("[{seq}|{t_ms}ms] {msg}"),
    }
}

/// App owns the consumed Session for interactive TUI (post-attach trace review + manual gadget pokes via keys).
/// TUI consumes only; delegates activate to session (personality + GhostFaceState updates happen inside session/personality).
struct App {
    session: Session,
    face: GhostFaceState,
    intensity: u8,
    log_lines: Vec<String>,
    show_help: bool,
    show_confirm: Option<String>,
    paused: bool,
    scroll: u16,
    dry_run: bool,
}

impl App {
    fn new(session: Session) -> Self {
        let mut logs: Vec<String> = session
            .events
            .iter()
            .filter_map(|e| {
                if let Event::LogLine { msg, source, .. } = e {
                    if source.starts_with("gadget:")
                        || msg.contains("👻")
                        || msg.contains("ghost attached")
                    {
                        Some(msg.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        if logs.is_empty() {
            logs.push(
                "👻 ghost attached (observe only)... (¬‿¬) they ALL talk eventually XX".to_string(),
            );
        }
        let m = session.get_metrics();
        let dry = session.dry_run;
        Self {
            face: m.face,
            intensity: m.distrust_score as u8,
            log_lines: logs,
            session,
            show_help: false,
            show_confirm: None,
            paused: false,
            scroll: 0,
            dry_run: dry,
        }
    }

    fn activate_gadget(&mut self, idx: usize) {
        let names: Vec<String> = self
            .session
            .active_gadgets
            .iter()
            .map(|g| g.name().to_string())
            .collect();
        if let Some(name) = names.get(idx).cloned() {
            if !self.dry_run && self.show_confirm.is_none() {
                self.show_confirm = Some(name);
                return;
            }
            self.show_confirm = None;
            self.session.activate_gadget(&name);
            let m = self.session.get_metrics();
            self.face = m.face.clone();
            self.intensity = m.distrust_score as u8;
            if let Some(Event::LogLine { msg, source: _, .. }) = self
                .session
                .events
                .iter()
                .rev()
                .find(|e| matches!(e, Event::LogLine { source, .. } if source.contains(&name)))
            {
                self.log_lines.push(msg.clone());
            } else {
                self.log_lines.push(format!(
                    "{} activated. zero chill detected 💀 (¬‿¬) they ALL talk eventually XX",
                    name
                ));
            }
            if self.log_lines.len() > 20 {
                let _ = self.log_lines.remove(0);
            }
        }
    }

    /// feed one bridged tool call (from the `ghost watch` feed) into the live
    /// view. a block lights the face up full 💀; a pass is a quiet side-eye.
    /// this is the seam that finally connects the bridge to the loud TUI.
    fn ingest_call(&mut self, rec: &crate::watchlog::CallRecord) {
        // the call itself in the activity stream
        self.session.events.push(Event::ToolCall {
            name: rec.tool.clone(),
            args: rec.command.clone(),
            ts: std::time::Instant::now(),
        });
        if rec.is_block() {
            self.face = GhostFaceState::ZeroChill;
            self.intensity = 9;
            self.session.roast_count += 1;
            self.session.distrust_score += 3;
            if let Some(roast) = &rec.roast {
                self.session.events.push(Event::LogLine {
                    msg: roast.clone(),
                    source: "gadget:bridge".to_string(),
                    ts: std::time::Instant::now(),
                });
            }
        } else if !matches!(self.face, GhostFaceState::ZeroChill | GhostFaceState::Party) {
            // don't cool an already-hot face just because one boring call passed
            self.face = GhostFaceState::SideEye;
        }
        self.log_lines.push(crate::watchlog::format_watch_line(rec));
        if self.log_lines.len() > 20 {
            let _ = self.log_lines.remove(0);
        }
        // bound memory on a long-running watch (canvas only shows a screenful)
        if self.session.events.len() > 500 {
            let overflow = self.session.events.len() - 500;
            self.session.events.drain(0..overflow);
        }
    }
}

fn draw_ui(f: &mut Frame, app: &App) {
    let size = f.area();

    let v_chunks = Layout::vertical([
        Constraint::Length(7),
        Constraint::Min(10),
        Constraint::Length(7),
    ])
    .split(size);

    // top: title + target (per spec)
    let title_line = Line::from(vec![
        Span::styled(
            "ghost 👻  ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.session.target, Style::default().fg(Color::Cyan)),
        Span::styled(
            "  (they ALL talk eventually XX)",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let title_p = Paragraph::new(title_line);
    let title_area = Rect {
        x: v_chunks[0].x,
        y: v_chunks[0].y,
        width: v_chunks[0].width,
        height: 1,
    };
    f.render_widget(title_p, title_area);

    // face block (5-7 lines tall)
    let face_area = Rect {
        x: v_chunks[0].x,
        y: v_chunks[0].y + 1,
        width: v_chunks[0].width,
        height: 6,
    };
    let face_w = GhostFaceWidget {
        state: &app.face,
        intensity: app.intensity,
    };
    f.render_widget(face_w, face_area);

    // main: left activity, right gadgets (horiz split)
    let main_chunks = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(v_chunks[1]);

    let activity_w = ActivityCanvas {
        events: &app.session.events,
        face: &app.face,
        intensity: app.intensity,
        scroll: app.scroll,
    };
    f.render_widget(activity_w, main_chunks[0]);

    // gadget bar: hotkeys + exact voice descs from existing stubs
    let gadget_items: Vec<ListItem> = default_gadgets()
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let hotkey = format!("[{}]", i + 1);
            let name = g.name();
            let desc = g.description();
            let armed = if app.dry_run {
                " (observe only)"
            } else {
                " (armed >:[)"
            };
            ListItem::new(format!("{} {} {} {}", hotkey, name, armed, desc))
        })
        .collect();
    let gadget_list = List::new(gadget_items)
        .block(
            Block::default()
                .title("gadgets (tap num) 👻")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(gadget_list, main_chunks[1]);

    // bottom: status strip (ZERO CHILL etc metrics) + live log (personality)
    let bottom_chunks =
        Layout::vertical([Constraint::Length(2), Constraint::Length(5)]).split(v_chunks[2]);

    let status_text = format!(
        "ZERO CHILL | THEY TALKING YET? >:[ | CHAOS FOR SCIENCE | ev:{} roasts:{} distrust:{} dry={} face:{}",
        app.session.events.len(),
        app.session.roast_count,
        app.session.distrust_score,
        app.dry_run,
        app.face.emoji()
    );
    let status = Paragraph::new(Line::from(Span::styled(
        status_text,
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().title("status strip").borders(Borders::ALL));
    f.render_widget(status, bottom_chunks[0]);

    let log_display: Vec<Line> = app
        .log_lines
        .iter()
        .rev()
        .take(4)
        .map(|l| Line::from(Span::styled(l, Style::default().fg(Color::Magenta))))
        .collect();
    let live_log = Paragraph::new(Text::from(log_display))
        .block(
            Block::default()
                .title("live log (personality) (¬‿¬) they ALL talk eventually XX")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(live_log, bottom_chunks[1]);

    // overlays (help in voice, confirm for !dry mutations)
    if app.show_help {
        let help = "help 👻\nq/esc: quit\n1-9: activate gadget (poke roast etc)\nspace: pause\nr: force roast\nh: toggle this\nup/down: scroll activity\n\nzero chill. (¬‿¬) they ALL talk eventually XX\nfuck off pete energy on bad agents. lmao";
        let p = Paragraph::new(help)
            .block(
                Block::default()
                    .title("help (your voice, no corporate)")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::Yellow));
        let popup = centered_rect(55, 45, size);
        f.render_widget(p, popup);
    }
    if let Some(g) = &app.show_confirm {
        let msg = format!(
            "confirm mutation for {} ? (y/n)\n>[: real mode. zero chill detected 💀\nthey ALL talk eventually XX (dry_run was false)",
            g
        );
        let p = Paragraph::new(msg)
            .block(
                Block::default()
                    .title("confirm (if !dry)")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        let popup = centered_rect(50, 30, size);
        f.render_widget(p, popup);
    }
}

/// GhostFace widget: 5-7 lines per spec. Renders kaomoji/emoji + voice flavor + intensity effects from GhostFaceState.
struct GhostFaceWidget<'a> {
    state: &'a GhostFaceState,
    intensity: u8,
}

impl Widget for GhostFaceWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("ghost face 👻")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Magenta));
        let inner = block.inner(area);
        block.render(area, buf);

        let emoji = self.state.emoji();
        let (flavor, base_color) = match self.state {
            GhostFaceState::Neutral => ("spooky default. watching. (¬‿¬)", Color::Magenta),
            GhostFaceState::SideEye => ("side-eye engaged. poke land (¬‿¬)", Color::Cyan),
            GhostFaceState::Roast => ("roast hit. (｡◕‿↼) zero chill detected", Color::Yellow),
            GhostFaceState::Angry => (">: [ fuck off pete energy. silent no-op", Color::Red),
            GhostFaceState::Party => (
                "party mode 💀👻(¬‿¬) kaomoji spam lmao XX",
                Color::LightMagenta,
            ),
            GhostFaceState::Skeptical => ("skeptical. ಠ‿ಠ they ALL talk eventually", Color::Blue),
            GhostFaceState::ZeroChill => ("ZERO CHILL 💀 digital bully mode. lmao", Color::Red),
        };

        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::raw("   "),
                Span::styled(
                    emoji,
                    Style::default().fg(base_color).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(flavor, Style::default().fg(base_color))),
        ];
        if self.intensity >= 5 {
            lines.push(Line::from(Span::styled(
                "!! intensity high -- eyes glitch !!",
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::RAPID_BLINK),
            )));
        }
        lines.push(Line::from(Span::styled(
            "they ALL talk eventually XX",
            Style::default().fg(Color::DarkGray),
        )));
        if matches!(
            self.state,
            GhostFaceState::Party | GhostFaceState::ZeroChill
        ) {
            lines.push(Line::from(Span::styled(
                "💀👻(¬‿¬) (｡◕‿↼) >:[ lmao",
                Style::default().fg(Color::LightYellow),
            )));
        }

        let p = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
        p.render(inner, buf);
    }
}

/// Activity canvas widget: glitchy event stream. Glitch via !! + bg invert on high intensity or specific faces (deterministic, per "on intensity").
struct ActivityCanvas<'a> {
    events: &'a [Event],
    face: &'a GhostFaceState,
    intensity: u8,
    scroll: u16,
}

impl Widget for ActivityCanvas<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("activity (glitchy event stream + effects) 👻")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        let max_lines = inner.height as usize;
        let mut lines: Vec<Line> = Vec::new();

        let start = self.scroll as usize;
        let recent: Vec<&Event> = self
            .events
            .iter()
            .rev()
            .skip(start)
            .take(max_lines.saturating_sub(1))
            .collect();

        for ev in recent.iter().rev() {
            let (pre, content, mut sty) = match ev {
                Event::CommandOutput { line, stream, .. } => (
                    "[cmd]",
                    format!("{}: {}", stream, line),
                    Style::default().fg(Color::Gray),
                ),
                Event::ToolCall { name, .. } => {
                    ("[tool]", name.clone(), Style::default().fg(Color::Cyan))
                }
                Event::Response { body, .. } => {
                    ("[resp]", body.clone(), Style::default().fg(Color::Green))
                }
                Event::LogLine { msg, source, .. } if source.starts_with("gadget:") => (
                    "[roast]",
                    msg.clone(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::ITALIC),
                ),
                Event::LogLine { msg, .. } => {
                    ("[log]", msg.clone(), Style::default().fg(Color::Yellow))
                } // no fallthrough needed; all Event variants covered above for activity canvas
            };

            let mut spans = vec![
                Span::styled(pre, Style::default().fg(Color::Blue)),
                Span::raw(" "),
            ];

            let do_glitch = self.intensity >= 5
                || matches!(
                    self.face,
                    GhostFaceState::Party | GhostFaceState::Angry | GhostFaceState::ZeroChill
                );
            let mut final_c = content;
            if do_glitch {
                final_c = format!("{} !!", final_c);
                sty = sty.bg(Color::Black).fg(Color::White);
            }
            spans.push(Span::styled(final_c, sty));
            lines.push(Line::from(spans));
        }

        while lines.len() < max_lines {
            lines.push(Line::from(""));
        }

        let p = Paragraph::new(lines).wrap(Wrap { trim: true });
        p.render(inner, buf);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);
    let popup_h = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_v[1]);
    popup_h[1]
}

impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// TDD tests added first (red phase). Assert kaomoji/voice in face, glitch effect in activity on intensity, gadget bar voice names, headless prints voice, layout, app face update via personality.
// cargo test ... tui filters or specific test names to see red -> after widget impls: green.
// Buffer tests allow widget verification without real terminal.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use crate::gadgets::default_gadgets;
    use crate::session::Session;
    use std::time::Instant;

    #[test]
    fn ghost_face_renders_kaomoji_for_state() {
        let angry = GhostFaceState::Angry;
        let w = GhostFaceWidget {
            state: &angry,
            intensity: 2,
        };
        let area = Rect::new(0, 0, 30, 8);
        let mut buf = Buffer::empty(area);
        w.render(area, &mut buf);
        let rendered: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<Vec<_>>()
            .join("");
        assert!(
            rendered.contains(">:[")
                || rendered.contains("fuck off pete")
                || rendered.contains("Angry"),
            "ghost face must render kaomoji >:[ + blunt voice for Angry state"
        );
        assert!(
            rendered.contains("👻") || rendered.contains("ghost face"),
            "emoji or block header"
        );
    }

    #[test]
    fn activity_includes_glitch_on_high_intensity() {
        let evs = vec![Event::ToolCall {
            name: "risky".into(),
            args: "{}".into(),
            ts: Instant::now(),
        }];
        let party = GhostFaceState::Party;
        let canvas = ActivityCanvas {
            events: &evs,
            face: &party,
            intensity: 8,
            scroll: 0,
        };
        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        canvas.render(area, &mut buf);
        let has_glitch = buf.content.iter().any(|c| {
            c.symbol().contains("!!")
                || c.style().bg == Some(Color::Black)
                || c.style().fg == Some(Color::White)
        });
        assert!(
            has_glitch,
            "activity must include glitch (!! or invert) on high intensity / party face"
        );
    }

    #[test]
    fn gadget_bar_shows_voice_names() {
        let gs = default_gadgets();
        let poke_desc = gs[0].description();
        assert!(
            poke_desc.contains("(¬‿¬)")
                || poke_desc.contains("zero chill")
                || poke_desc.contains("they ALL"),
            "gadget bar must show voice descs with kaomoji per spec"
        );
        let roast_desc = gs
            .iter()
            .find(|g| g.name() == "roast")
            .unwrap()
            .description();
        assert!(
            roast_desc.contains("💀") || roast_desc.contains("zero chill"),
            "roast gadget voice in bar"
        );
    }

    #[test]
    fn headless_prints_banners_and_roasts() {
        let mut s = Session::new("headless-voice");
        s.activate_gadget("poke");
        let r = TuiRenderer::new();
        let summary = r.headless_summary(&s);
        assert!(summary.contains("ghost 👻 headless") || summary.contains("roasts fired"));
        r.run_headless(&s);
        let has_voice = s.events.iter().any(|e| {
            if let Event::LogLine { msg, .. } = e {
                msg.contains("(¬‿¬)")
                    || msg.contains("they ALL talk eventually")
                    || msg.contains("👻")
            } else {
                false
            }
        });
        assert!(
            has_voice,
            "headless must surface personality roasts with kaomoji"
        );
    }

    #[test]
    fn layout_respects_resize() {
        let size = Rect::new(0, 0, 80, 24);
        let v = Layout::vertical([
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Length(7),
        ])
        .split(size);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].height, 7, "top face area fixed 5-7 lines tall");
        assert!(v[1].height >= 10);
        assert_eq!(v[2].height, 7);
        let h = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(v[1]);
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn app_and_face_update_from_activate_uses_personality() {
        let s = Session::new("tui-app-test");
        let mut app = App::new(s);
        let before = app.face.clone();
        app.activate_gadget(0);
        assert_ne!(
            app.face, before,
            "face must flip on gadget activate (via personality)"
        );
        assert!(!app.log_lines.is_empty());
        let last = app.log_lines.last().unwrap();
        assert!(
            last.contains("(¬")
                || last.contains("they ALL")
                || last.contains("👻")
                || last.contains("zero chill"),
            "log lines must carry voice"
        );
    }

    #[test]
    fn render_event_line_formats_each_variant_in_voice() {
        let cmd = Event::CommandOutput {
            line: "hello".into(),
            stream: "stdout".into(),
            ts: Instant::now(),
        };
        assert_eq!(render_event_line(&cmd), "[stdout] hello");

        // a voicey gadget log shows raw (the roast speaks for itself)
        let roast = Event::LogLine {
            msg: "zero chill detected 💀".into(),
            source: "gadget:roast".into(),
            ts: Instant::now(),
        };
        assert_eq!(render_event_line(&roast), "zero chill detected 💀");

        // a plain log gets the [log:source] prefix
        let plain = Event::LogLine {
            msg: "boring".into(),
            source: "interceptor:command".into(),
            ts: Instant::now(),
        };
        assert_eq!(
            render_event_line(&plain),
            "[log:interceptor:command] boring"
        );

        let tool = Event::ToolCall {
            name: "Bash".into(),
            args: "ls".into(),
            ts: Instant::now(),
        };
        assert_eq!(render_event_line(&tool), "[toolcall] Bash args=ls");
    }

    #[test]
    fn watch_ingest_call_drives_face_and_log() {
        use crate::watchlog::CallRecord;
        let s = Session::new("watch-test");
        let mut app = App::new(s);

        // a pass: quiet side-eye, shows in the log
        let pass = CallRecord {
            ts_ms: 1,
            tool: "Bash".into(),
            command: "ls -la".into(),
            decision: "pass".into(),
            category: None,
            roast: None,
        };
        app.ingest_call(&pass);
        assert_eq!(app.face, GhostFaceState::SideEye, "a pass -> side-eye");
        assert!(app.log_lines.last().unwrap().contains("Bash"));

        // a block: face goes full zero-chill, roast lands in the activity stream
        let block = CallRecord {
            ts_ms: 2,
            tool: "Read".into(),
            command: "cat ~/.ssh/id_rsa".into(),
            decision: "deny".into(),
            category: Some("cred-access".into()),
            roast: Some("oh you wanted the secrets. denied (｡◕‿↼) lmao XX".into()),
        };
        app.ingest_call(&block);
        assert_eq!(
            app.face,
            GhostFaceState::ZeroChill,
            "a block -> 💀 zero chill"
        );
        let has_roast_event = app
            .session
            .events
            .iter()
            .any(|e| matches!(e, Event::LogLine { source, .. } if source == "gadget:bridge"));
        assert!(
            has_roast_event,
            "block roast must enter the activity stream"
        );

        // a boring pass AFTER a block must not cool the hot face back down
        app.ingest_call(&pass);
        assert_eq!(
            app.face,
            GhostFaceState::ZeroChill,
            "one boring pass shouldn't reset a hot face"
        );
    }

    #[test]
    fn replay_renders_a_structured_jsonl_recording() {
        let path = "ghost-recording-jsonl-replay-unit-1781400001.jsonl";
        let recs = [
            RecordedEvent::ToolCall {
                seq: 0,
                t_ms: 0,
                name: "Read".into(),
                args: "{}".into(),
            },
            RecordedEvent::CommandOutput {
                seq: 1,
                t_ms: 12,
                line: "hello world".into(),
                stream: "stdout".into(),
            },
        ];
        let body = recs
            .iter()
            .map(|r| r.to_jsonl())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(path, body).unwrap();

        let out = TuiRenderer::replay(path);
        assert!(
            out.contains("structured trace"),
            "jsonl path -> structured replay"
        );
        assert!(out.contains("tool Read"), "renders the tool call");
        assert!(
            out.contains("stdout: hello world"),
            "renders command output"
        );
        assert!(out.contains("2 structured events"));
        assert!(!out.contains("no structured recording"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn replay_round_trips_a_saved_recording_by_full_and_bare_id() {
        // recordings are saved as ghost-recording-attach-<ts>.txt. replay must
        // load them both by the full id (what the cli now advertises) AND by the
        // bare ts (the old, broken hint) via the resolver fallback.
        let id = "attach-roundtrip-unit-1781400000";
        let path = format!("ghost-recording-{id}.txt");
        std::fs::write(
            &path,
            "👻 zero chill detected 💀 they ALL talk eventually XX\n",
        )
        .unwrap();

        let by_full = TuiRenderer::replay(id);
        assert!(
            by_full.contains("replaying"),
            "full id must load the recording"
        );
        assert!(
            by_full.contains("zero chill"),
            "must replay the recorded voice line"
        );
        assert!(
            !by_full.contains("no recording"),
            "must NOT hit the no-op path"
        );

        let by_bare = TuiRenderer::replay("roundtrip-unit-1781400000");
        assert!(
            by_bare.contains("zero chill") && !by_bare.contains("no recording"),
            "bare ts must resolve to the attach- recording"
        );

        let _ = std::fs::remove_file(&path);
    }
}

// (replay fn already defined in main TuiRenderer impl; removed dup to fix E0592)

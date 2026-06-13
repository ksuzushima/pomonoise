use std::io::{self, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use ratatui::Terminal;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, LineGauge, Paragraph};
use tui_big_text::{BigText, PixelSize};

use crate::app::{AppState, NoiseType, Phase, Status, TransitionEvent};
use crate::audio::AudioEngine;

const COLOR_WORK: Color = Color::Rgb(0, 255, 136);
const COLOR_BREAK: Color = Color::Rgb(100, 180, 255);
const COLOR_PAUSED: Color = Color::Rgb(255, 100, 100);
const COLOR_DIM: Color = Color::DarkGray;

/// Event-poll timeout. Bounds key-input latency without busy-looping; the
/// actual redraw cadence is driven by `RenderSnapshot` changes, not by this.
const POLL_TIMEOUT: Duration = Duration::from_millis(100);
/// Volume increment applied per keypress.
const VOLUME_STEP: f32 = 0.05;

/// Snapshot of everything that affects the rendered frame. The UI loop redraws
/// (and updates the terminal title) only when this changes, so an idle timer
/// costs one cheap comparison per poll instead of a full repaint ten times a
/// second.
#[derive(PartialEq)]
struct RenderSnapshot {
    remaining_secs: u64,
    phase: Phase,
    status: Status,
    set_index: u32,
    noise: NoiseType,
    volume: f32,
    help_visible: bool,
}

impl RenderSnapshot {
    fn capture(state: &AppState, help_visible: bool) -> Self {
        Self {
            remaining_secs: state.remaining.as_secs(),
            phase: state.phase,
            status: state.status,
            set_index: state.set_index,
            noise: state.noise,
            volume: state.volume,
            help_visible,
        }
    }
}

fn phase_color(phase: Phase) -> Color {
    match phase {
        Phase::Work => COLOR_WORK,
        Phase::Break => COLOR_BREAK,
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self, Box<dyn std::error::Error>> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), SetTitle(""));
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Run the TUI event loop.
pub fn run(mut state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let _terminal_guard = TerminalGuard::enter()?;
    let stdout = stdout();
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut audio = AudioEngine::new()?;
    audio.start(state.noise, state.volume);

    let mut last_tick = Instant::now();
    let mut help_visible = false;
    let mut last_snapshot: Option<RenderSnapshot> = None;

    loop {
        // Redraw (and re-title) only when something visible changed. Between
        // one-second ticks nothing changes, so an idle loop just polls.
        let snapshot = RenderSnapshot::capture(&state, help_visible);
        if last_snapshot.as_ref() != Some(&snapshot) {
            let title = format!(
                "{} {} - pomonoise",
                state.remaining_display(),
                state.phase_display()
            );
            execute!(io::stdout(), SetTitle(&title))?;
            terminal.draw(|frame| render(frame, &state, help_visible))?;
            last_snapshot = Some(snapshot);
        }

        // Poll events, bounding input latency without busy-looping.
        if event::poll(POLL_TIMEOUT)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('?') | KeyCode::Char('h') => {
                        help_visible = !help_visible;
                    }
                    KeyCode::Char(' ') => {
                        state.toggle_pause();
                        if state.status == Status::Paused {
                            audio.stop();
                        } else if state.phase == Phase::Work {
                            audio.start(state.noise, state.volume);
                        }
                    }
                    KeyCode::Char('n') => {
                        state.cycle_noise();
                        if state.status == Status::Running && state.phase == Phase::Work {
                            audio.set_noise(state.noise);
                        }
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Up => {
                        state.adjust_volume(VOLUME_STEP);
                        audio.set_volume(state.volume);
                    }
                    KeyCode::Char('-') | KeyCode::Down => {
                        state.adjust_volume(-VOLUME_STEP);
                        audio.set_volume(state.volume);
                    }
                    KeyCode::Char('s') => {
                        let event = state.skip();
                        handle_transition_event(event, &state, &mut audio);
                    }
                    KeyCode::Char('r') => {
                        state.reset();
                        // Back in Work/Running, so resume noise immediately.
                        audio.start(state.noise, state.volume);
                    }
                    _ => {}
                },
                // Force a repaint on the next iteration after a resize.
                Event::Resize(_, _) => last_snapshot = None,
                _ => {}
            }
        }

        // Tick timer.
        let now = Instant::now();
        if state.status == Status::Running {
            let delta = now.duration_since(last_tick);
            let event = state.tick(delta);
            handle_transition_event(event, &state, &mut audio);
        }
        last_tick = now;

        if state.finished {
            audio.stop(); // idempotent safety if finished state is externally injected
            terminal.draw(render_completion)?;
            // Wait for any key press (ignoring key-release events on Windows).
            loop {
                if event::poll(POLL_TIMEOUT)?
                    && let Event::Key(key) = event::read()?
                    && key.kind == KeyEventKind::Press
                {
                    break;
                }
            }
            break;
        }
    }

    Ok(())
}

fn handle_transition_event(event: TransitionEvent, state: &AppState, audio: &mut AudioEngine) {
    match event {
        TransitionEvent::None => {}
        TransitionEvent::PhaseChanged { to, .. } => {
            let _ = execute!(io::stdout(), Print("\x07"));
            if to == Phase::Break {
                audio.stop();
            } else if to == Phase::Work && state.status == Status::Running {
                audio.start(state.noise, state.volume);
            }
        }
        TransitionEvent::Finished => {
            audio.stop();
            let _ = execute!(io::stdout(), Print("\x07"));
        }
    }
}

fn render(frame: &mut ratatui::Frame, state: &AppState, help_visible: bool) {
    let area = frame.area();
    let color = phase_color(state.phase);

    // Timer text
    let timer_text = state.remaining_display();
    let timer_color = if state.status == Status::Paused {
        COLOR_PAUSED
    } else if state.remaining.as_secs() < 60 {
        color
    } else {
        Color::White
    };

    let big_text = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .style(Style::new().fg(timer_color))
        .lines(vec![timer_text.into()])
        .centered()
        .build();

    // Progress bar
    let gauge = LineGauge::default()
        .ratio(state.progress().clamp(0.0, 1.0))
        .filled_symbol(symbols::line::THICK_HORIZONTAL)
        .unfilled_symbol(symbols::line::THICK_HORIZONTAL)
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::Rgb(60, 60, 60)));

    // Phase + session dots
    let phase_label = match state.phase {
        Phase::Work => "WORK",
        Phase::Break => "BREAK",
    };
    let phase_text = format!("{phase_label} {}/{}", state.set_index, state.sets.get());
    let dots = session_dots(state.set_index, state.sets.get());

    let mut phase_spans = vec![
        ratatui::text::Span::styled(
            phase_text,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        ratatui::text::Span::raw("   "),
        ratatui::text::Span::styled(dots, Style::default().fg(color)),
    ];
    if state.status == Status::Paused {
        phase_spans.push(ratatui::text::Span::raw("   "));
        phase_spans.push(ratatui::text::Span::styled(
            "PAUSED",
            Style::default()
                .fg(COLOR_PAUSED)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let phase_widget =
        Paragraph::new(ratatui::text::Line::from(phase_spans)).alignment(Alignment::Center);

    // Noise + volume bar
    let filled = (state.volume * 10.0).round() as usize;
    let vol_bar: String = "▮".repeat(filled) + &"▯".repeat(10 - filled);
    let noise_spans = vec![
        ratatui::text::Span::styled("♪ ", Style::default().fg(color)),
        ratatui::text::Span::styled(state.noise.as_str(), Style::default().fg(Color::White)),
        ratatui::text::Span::raw("  "),
        ratatui::text::Span::styled(&vol_bar, Style::default().fg(color)),
    ];
    let noise_widget =
        Paragraph::new(ratatui::text::Line::from(noise_spans)).alignment(Alignment::Center);

    // Footer
    let footer =
        Paragraph::new("Space pause │ n noise │ ±vol │ s skip │ r reset │ ? help │ q quit")
            .style(Style::default().fg(COLOR_DIM))
            .alignment(Alignment::Center);

    // Layout
    let vertical = Layout::vertical([
        Constraint::Fill(1),   // top spacer
        Constraint::Length(4), // big text timer
        Constraint::Length(1), // spacer
        Constraint::Length(1), // progress bar
        Constraint::Length(1), // spacer
        Constraint::Length(1), // phase + dots
        Constraint::Length(1), // noise + volume
        Constraint::Fill(1),   // bottom spacer
        Constraint::Length(1), // footer
    ])
    .split(area);

    // Horizontal centering for progress bar
    let gauge_area = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ])
    .split(vertical[3]);

    frame.render_widget(big_text, vertical[1]);
    frame.render_widget(gauge, gauge_area[1]);
    frame.render_widget(phase_widget, vertical[5]);
    frame.render_widget(noise_widget, vertical[6]);
    frame.render_widget(footer, vertical[8]);

    if help_visible {
        render_help(frame, area);
    }
}

fn session_dots(set_index: u32, sets: u32) -> String {
    const MAX_VISIBLE_DOTS: u32 = 20;
    if sets == 0 {
        return "—".to_string();
    }

    if sets <= MAX_VISIBLE_DOTS {
        return (1..=sets)
            .map(|i| if i <= set_index { "●" } else { "○" })
            .collect::<Vec<_>>()
            .join(" ");
    }

    let visible_completed = (((set_index.min(sets) as f64 / sets as f64) * MAX_VISIBLE_DOTS as f64)
        .round() as u32)
        .clamp(0, MAX_VISIBLE_DOTS);
    let dots = (1..=MAX_VISIBLE_DOTS)
        .map(|i| if i <= visible_completed { "●" } else { "○" })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{dots} +{}", sets - MAX_VISIBLE_DOTS)
}

/// Build one "key   description" row for the help overlay.
fn help_line(key: &'static str, desc: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{key:<11}"),
            Style::default().fg(COLOR_WORK).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}

/// Compute a `width`×`height` rectangle centered within `area`, clamped to fit.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// Render the help overlay as a centered popup over the current frame.
fn render_help(frame: &mut ratatui::Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        help_line("Space", "pause / resume"),
        help_line("n", "cycle noise"),
        help_line("+ / = / Up", "volume up"),
        help_line("- / Down", "volume down"),
        help_line("s", "skip phase"),
        help_line("r", "reset session"),
        help_line("? / h", "toggle this help"),
        help_line("q", "quit"),
        Line::from(""),
    ];

    let height = lines.len() as u16 + 2; // + top/bottom border
    let popup = centered_rect(36, height, area);

    let block = Block::default()
        .title(" keys ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_WORK));

    frame.render_widget(Clear, popup);
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

fn render_completion(frame: &mut ratatui::Frame) {
    let area = frame.area();

    let done_text = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .style(Style::new().fg(COLOR_WORK))
        .lines(vec!["DONE".into()])
        .centered()
        .build();

    let subtitle = Paragraph::new("Session complete")
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);

    let hint = Paragraph::new("press any key to exit")
        .style(Style::default().fg(COLOR_DIM))
        .alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(4), // DONE big text
        Constraint::Length(1), // spacer
        Constraint::Length(1), // subtitle
        Constraint::Length(1), // hint
        Constraint::Fill(1),
    ])
    .split(area);

    frame.render_widget(done_text, vertical[1]);
    frame.render_widget(subtitle, vertical[3]);
    frame.render_widget(hint, vertical[4]);
}

#[cfg(test)]
mod tests {
    use super::session_dots;

    #[test]
    fn session_dots_small_set_count() {
        assert_eq!(session_dots(2, 4), "● ● ○ ○");
    }

    #[test]
    fn session_dots_large_set_count_is_capped() {
        let dots = session_dots(40, 100);
        assert!(dots.contains("+80"));
    }

    #[test]
    fn session_dots_zero_sets_fallback() {
        assert_eq!(session_dots(1, 0), "—");
    }
}

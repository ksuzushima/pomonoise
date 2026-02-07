use std::io::{self, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use ratatui::Terminal;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::widgets::{LineGauge, Paragraph};
use tui_big_text::{BigText, PixelSize};

use crate::app::{AppState, Phase, Status};
use crate::audio::AudioEngine;

const COLOR_WORK: Color = Color::Rgb(0, 255, 136);
const COLOR_BREAK: Color = Color::Rgb(100, 180, 255);
const COLOR_PAUSED: Color = Color::Rgb(255, 100, 100);
const COLOR_DIM: Color = Color::DarkGray;

fn phase_color(phase: Phase) -> Color {
    match phase {
        Phase::Work => COLOR_WORK,
        Phase::Break => COLOR_BREAK,
    }
}

/// Run the TUI event loop.
pub fn run(mut state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut audio = AudioEngine::new();
    audio.start(state.noise, state.volume);

    let mut last_tick = Instant::now();

    loop {
        // Update terminal title
        let title = format!(
            "{} {} - pomonoise",
            state.remaining_display(),
            state.phase_display()
        );
        execute!(io::stdout(), SetTitle(&title))?;

        // Render
        terminal.draw(|frame| render(frame, &state))?;

        // Poll events with ~100ms timeout
        let timeout = Duration::from_millis(100);
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => break,
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
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    state.adjust_volume(0.05);
                    audio.set_volume(state.volume);
                }
                KeyCode::Char('-') => {
                    state.adjust_volume(-0.05);
                    audio.set_volume(state.volume);
                }
                KeyCode::Char('s') => {
                    let old_phase = state.phase;
                    state.skip();
                    handle_phase_change(old_phase, &state, &mut audio);
                }
                _ => {}
            }
        }

        // Tick timer
        if state.status == Status::Running {
            let now = Instant::now();
            let delta = now.duration_since(last_tick);
            let phase_before = state.phase;
            state.tick(delta);

            if state.phase != phase_before {
                // Bell on phase transition
                let _ = execute!(io::stdout(), Print("\x07"));
                handle_phase_change(phase_before, &state, &mut audio);
            }
        }
        last_tick = Instant::now();

        if state.finished {
            audio.stop();
            let _ = execute!(io::stdout(), Print("\x07"));
            // Show completion screen
            terminal.draw(render_completion)?;
            // Wait for any key
            loop {
                if event::poll(Duration::from_millis(100))?
                    && let Event::Key(_) = event::read()?
                {
                    break;
                }
            }
            break;
        }
    }

    // Cleanup
    audio.stop();
    execute!(io::stdout(), SetTitle(""))?;
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn handle_phase_change(old_phase: Phase, state: &AppState, audio: &mut AudioEngine) {
    if state.phase != old_phase || state.finished {
        if state.phase == Phase::Break || state.finished {
            audio.stop();
        } else if state.phase == Phase::Work && state.status == Status::Running {
            audio.start(state.noise, state.volume);
        }
    }
}

fn render(frame: &mut ratatui::Frame, state: &AppState) {
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
    let dots: String = (1..=state.sets)
        .map(|i| if i <= state.set_index { "●" } else { "○" })
        .collect::<Vec<_>>()
        .join(" ");

    let mut phase_spans = vec![
        ratatui::text::Span::styled(
            phase_label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        ratatui::text::Span::raw("   "),
        ratatui::text::Span::styled(&dots, Style::default().fg(color)),
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
    let footer = Paragraph::new("Space pause │ n noise │ ±vol │ s skip │ q quit")
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

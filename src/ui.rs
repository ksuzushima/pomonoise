use std::io::{self, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;

use crate::app::{AppState, Phase, Status};
use crate::audio::AudioEngine;

/// Run the TUI event loop.
pub fn run(mut state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut audio = AudioEngine::new();
    // Start playing initial noise since we begin in Work/Running state
    audio.start(state.noise, state.volume);

    let mut last_tick = Instant::now();

    loop {
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
                    if state.phase != old_phase || state.finished {
                        if state.phase == Phase::Break || state.finished {
                            audio.stop();
                        } else if state.phase == Phase::Work && state.status == Status::Running {
                            audio.start(state.noise, state.volume);
                        }
                    }
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

            // Handle phase transitions
            if state.phase != phase_before {
                if state.phase == Phase::Break {
                    audio.stop();
                } else if state.phase == Phase::Work && state.status == Status::Running {
                    audio.start(state.noise, state.volume);
                }
            }
        }
        last_tick = Instant::now();

        if state.finished {
            // Render one final frame showing finished state
            terminal.draw(|frame| render(frame, &state))?;
            break;
        }
    }

    // Cleanup
    audio.stop();
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}

fn render(frame: &mut ratatui::Frame, state: &AppState) {
    let area = frame.area();

    let phase_color = match state.phase {
        Phase::Work => Color::Green,
        Phase::Break => Color::Yellow,
    };

    let time_text = if state.finished {
        "Done!".to_string()
    } else {
        state.remaining_display()
    };

    let time_widget = Paragraph::new(time_text)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);

    let phase_widget = Paragraph::new(state.phase_display())
        .style(Style::default().fg(phase_color))
        .alignment(Alignment::Center);

    let noise_widget = Paragraph::new(state.noise_display())
        .style(Style::default())
        .alignment(Alignment::Center);

    let status_text = if state.finished {
        String::new()
    } else if state.status == Status::Paused {
        "PAUSED".to_string()
    } else {
        String::new()
    };

    let status_widget = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center);

    let footer_widget = Paragraph::new("Space pause | n noise | +/- volume | s skip | q quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    // Layout: vertically centered with content in the middle
    let vertical = Layout::vertical([
        Constraint::Fill(1),   // top spacer
        Constraint::Length(1), // time
        Constraint::Length(1), // phase
        Constraint::Length(1), // noise
        Constraint::Length(1), // status (paused indicator)
        Constraint::Fill(1),   // bottom spacer
        Constraint::Length(1), // footer
    ])
    .split(area);

    frame.render_widget(time_widget, vertical[1]);
    frame.render_widget(phase_widget, vertical[2]);
    frame.render_widget(noise_widget, vertical[3]);
    frame.render_widget(status_widget, vertical[4]);
    frame.render_widget(footer_widget, vertical[6]);
}

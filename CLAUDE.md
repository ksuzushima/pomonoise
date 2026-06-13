# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
cargo build                # Debug build
cargo build --release      # Release build
cargo run --release        # Build and run
cargo check                # Fast compile check (no codegen)

cargo test                 # Run all 65 tests
cargo test app::           # Run only app module tests (36 tests)
cargo test audio::noise    # Run only noise generator tests (13 tests)
cargo test parse_          # Run only parsing tests
cargo test -- --nocapture  # Show stdout in test output

cargo fmt --check          # Check formatting
cargo fmt                  # Apply formatting
cargo clippy -- -D warnings  # Lint with warnings as errors
```

Quality gate (run all three before committing):
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## Architecture

**Data flow:** `main.rs` (CLI parse) → creates `AppState` → passes to `ui::run()` → TUI event loop drives both `AppState` mutations and `AudioEngine` control.

### Modules

- **`main.rs`** — CLI argument parsing with `clap`. Converts duration strings (`25m`, `300s`) and noise type strings into typed values, then hands off to `ui::run()`.
- **`app.rs`** — Pure state machine. `AppState` holds all timer state (`Phase`, `Status`, remaining time, set index, noise, volume). Key methods: `tick(delta)`, `toggle_pause()`, `skip()`, `reset()`, `cycle_noise()`, `adjust_volume()`, `progress()`. 36 unit tests covering state transitions, pause/resume, skip, reset, volume, noise cycling, display formatting, and progress edge cases.
- **`ui.rs`** — TUI event loop using `ratatui`/`crossterm`/`tui-big-text`. Polls keyboard events (~100ms timeout) and mutates `AppState`, but redraws (and re-titles) only when a `RenderSnapshot` of the visible state changes — an idle timer no longer repaints ten times a second. Filters for `KeyEventKind::Press` (no double input on Windows) and forces a repaint on `Resize`. Handles pause, skip, reset (`r`), noise cycling, volume (`+`/`-`/arrows), and a help overlay (`?`/`h`). Owns the `AudioEngine` instance and syncs it with state changes; also manages terminal title updates, bell notifications, and the completion screen.
- **`audio/mod.rs`** — `AudioEngine` wraps `rodio` (`OutputStream` + `Sink`). `new()` returns `Result` (graceful handling when no audio device is available). `start()` handles `Sink` creation failure silently (logs to stderr, continues without audio). Methods: `start(noise, volume)`, `stop()`, `set_volume()`, `set_noise()`.
- **`audio/noise.rs`** — Three noise generators implementing `rodio::Source` (mono, `SAMPLE_RATE` = 44100Hz, infinite): `WhiteNoiseSource` (uniform random), `PinkNoiseSource` (Voss-McCartney 16-row algorithm), `BrownNoiseSource` (leaky-integrated random walk `x = x * BROWN_LEAK + step`, clamped — the leak prevents DC drift and clamp pinning over long runs). 13 unit tests covering output range, Source metadata, statistical properties, DC-drift/clamp behavior, and random-walk characteristics.

### Key design decisions

- **Audio-UI sync rule:** Audio plays only during Work phase while Running. UI loop is responsible for calling `audio.start()`/`audio.stop()` on every phase transition, pause/resume, skip, and reset. The `handle_transition_event()` helper centralizes the phase-transition part of this logic.
- **Break = silence:** No noise during break phases; no configuration for this.
- **State machine is pure:** `app.rs` has no side effects — all audio/terminal I/O lives in `ui.rs`. This makes the state logic fully unit-testable.
- **Noise sources use `SmallRng`** (not `ThreadRng`) because `rodio::Sink::append` requires `Send`.
- **`Duration::saturating_sub`** prevents underflow when tick delta exceeds remaining time.
- **`checked_mul`** in `parse_duration` prevents u64 overflow when converting minutes to seconds.
- **Panic hook** restores terminal state (raw mode off, alternate screen exit) on panic so the terminal is not left broken.
- **Snapshot-gated rendering:** Each loop iteration builds a `RenderSnapshot` (remaining *seconds*, phase, status, set index, noise, volume, help visibility) and redraws only when it differs from the previous one. Sub-second `progress()` changes don't trigger a repaint, so idle CPU/battery stays near zero without hurting input latency (the poll timeout is unchanged).
- **Leaky-integrated brown noise:** `BROWN_LEAK` (slightly below 1.0) pulls the random walk back toward zero each sample, preventing the DC drift and ±1.0 clamp pinning that a pure random walk accumulates over long sessions.
- **`KeyEventKind::Press` filtering:** crossterm emits both press and release events on Windows; the loop handles presses only so a single keystroke isn't processed twice.

### UI rendering

- **Color constants** are defined at the top of `ui.rs`: `COLOR_WORK` (mint green), `COLOR_BREAK` (soft blue), `COLOR_PAUSED` (soft red), `COLOR_DIM` (dark gray). Change these to adjust the theme.
- **Layout** uses vertically centered constraints with a horizontal sub-layout for the progress bar (60% width, centered).
- **tui-big-text compatibility:** `tui-big-text` 0.8 requires `ratatui` 0.30+ (uses `ratatui-core` 0.1). These versions must stay in sync.
- **Terminal title** is updated via `crossterm::terminal::SetTitle` only when the snapshot changes (alongside the redraw), and cleared on exit.
- **Completion screen** is a separate render function (`render_completion`) that blocks until any key is pressed.

## Rust Edition

Edition 2024 — requires Rust 1.88+ (pinned via `rust-toolchain.toml`).

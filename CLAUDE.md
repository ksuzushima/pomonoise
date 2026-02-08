# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
cargo build                # Debug build
cargo build --release      # Release build
cargo run --release        # Build and run
cargo check                # Fast compile check (no codegen)

cargo test                 # Run all 54 tests
cargo test app::           # Run only app module tests (37 tests)
cargo test audio::noise    # Run only noise generator tests (11 tests)
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
- **`app.rs`** — Pure state machine. `AppState` holds all timer state (`Phase`, `Status`, remaining time, set index, noise, volume). Key methods: `tick(delta)`, `toggle_pause()`, `skip()`, `cycle_noise()`, `adjust_volume()`, `progress()`. 37 unit tests covering state transitions, pause/resume, skip, volume, noise cycling, display formatting, and progress edge cases.
- **`ui.rs`** — TUI event loop using `ratatui`/`crossterm`/`tui-big-text`. Polls keyboard events (~100ms timeout), mutates `AppState`, syncs `AudioEngine` with state changes, and renders the display. Owns the `AudioEngine` instance. Also manages terminal title updates, bell notifications, and the completion screen.
- **`audio/mod.rs`** — `AudioEngine` wraps `rodio` (`OutputStream` + `Sink`). `new()` returns `Result` (graceful handling when no audio device is available). `start()` handles `Sink` creation failure silently (logs to stderr, continues without audio). Methods: `start(noise, volume)`, `stop()`, `set_volume()`, `set_noise()`.
- **`audio/noise.rs`** — Three noise generators implementing `rodio::Source` (mono, 44100Hz, infinite): `WhiteNoiseSource` (uniform random), `PinkNoiseSource` (Voss-McCartney 16-row algorithm), `BrownNoiseSource` (random walk ±0.05, clamped). 11 unit tests covering output range, Source metadata, statistical properties, and random-walk characteristics.

### Key design decisions

- **Audio-UI sync rule:** Audio plays only during Work phase while Running. UI loop is responsible for calling `audio.start()`/`audio.stop()` on every phase transition, pause/resume, and skip. The `handle_phase_change()` helper centralizes this logic.
- **Break = silence:** No noise during break phases; no configuration for this.
- **State machine is pure:** `app.rs` has no side effects — all audio/terminal I/O lives in `ui.rs`. This makes the state logic fully unit-testable.
- **Noise sources use `SmallRng`** (not `ThreadRng`) because `rodio::Sink::append` requires `Send`.
- **`Duration::saturating_sub`** prevents underflow when tick delta exceeds remaining time.
- **`checked_mul`** in `parse_duration` prevents u64 overflow when converting minutes to seconds.
- **Panic hook** restores terminal state (raw mode off, alternate screen exit) on panic so the terminal is not left broken.

### UI rendering

- **Color constants** are defined at the top of `ui.rs`: `COLOR_WORK` (mint green), `COLOR_BREAK` (soft blue), `COLOR_PAUSED` (soft red), `COLOR_DIM` (dark gray). Change these to adjust the theme.
- **Layout** uses vertically centered constraints with a horizontal sub-layout for the progress bar (60% width, centered).
- **tui-big-text compatibility:** `tui-big-text` 0.8 requires `ratatui` 0.30+ (uses `ratatui-core` 0.1). These versions must stay in sync.
- **Terminal title** is updated each frame via `crossterm::terminal::SetTitle` and cleared on exit.
- **Completion screen** is a separate render function (`render_completion`) that blocks until any key is pressed.

## Rust Edition

Edition 2024 — requires Rust 1.88+ (pinned via `rust-toolchain.toml`).

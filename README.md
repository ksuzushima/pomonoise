# pomonoise

A minimal CLI/TUI Pomodoro timer with white, pink, and brown noise. Stay focused with ambient noise during work sessions and enjoy silence during breaks.

## Features

- **3 noise types** — white, pink (Voss-McCartney), and brown (random walk)
- **Pomodoro cycle** — configurable work/break durations and number of sets
- **TUI** — big-text timer, progress bar, session dots, color-coded phases
- **Keyboard controls** — pause, skip, change noise, adjust volume, quit
- **Break = silence** — noise plays only during work phases
- **Terminal friendly** — updates window title, bell on phase change, graceful error handling

## Installation

Requires [Rust](https://www.rust-lang.org/tools/install) (1.88+). The `rust-toolchain.toml` will automatically select the correct version via rustup.

```bash
cargo install --path .
```

Or build and run directly:

```bash
cargo run --release
```

## Usage

```
pomonoise [OPTIONS]
```

### Options

| Option | Default | Description |
|---|---|---|
| `--work <DURATION>` | `25m` | Work phase duration |
| `--break <DURATION>` | `5m` | Break phase duration |
| `--sets <N>` | `4` | Number of work/break sets |
| `--noise <TYPE>` | `pink` | Noise type: `white`, `pink`, or `brown` |
| `--volume <FLOAT>` | `0.5` | Volume level (0.0 to 1.0) |

Durations accept minutes (`m`) or seconds (`s`) — e.g., `25m`, `1500s`.

### Examples

```bash
# Default: 25m work, 5m break, 4 sets, pink noise
pomonoise

# Short sessions with brown noise
pomonoise --work 10m --break 3m --sets 2 --noise brown

# Quiet white noise, single set
pomonoise --noise white --volume 0.2 --sets 1
```

## Key Bindings

| Key | Action |
|---|---|
| `Space` | Pause / Resume (audio follows) |
| `n` | Cycle noise type (white → pink → brown) |
| `+` / `=` | Volume up (+0.05) |
| `-` | Volume down (-0.05) |
| `s` | Skip to next phase |
| `q` | Quit |

## TUI Layout

```
              ██████╗ ██████╗ ██╗ ██████╗  ██████╗
              ╚════██╗██╔════╝ ██║██╔═══██╗██╔═══██╗
               █████╔╝██████╗  ██║██║   ██║██║   ██║
              ██╔═══╝ ╚════██║ ██║██║   ██║██║   ██║
              ███████╗██████╔╝ ██║╚██████╔╝╚██████╔╝
              ╚══════╝╚═════╝  ╚═╝ ╚═════╝  ╚═════╝

                    ━━━━━━━━━━━━━━━━━━━━━━

              WORK   ● ○ ○ ○
              ♪ pink  ▮▮▮▮▮▯▯▯▯▯

         Space pause │ n noise │ ±vol │ s skip │ q quit
```

## License

MIT

mod app;
mod audio;
mod ui;

use std::num::NonZeroU32;
use std::time::Duration;

use clap::{Parser, ValueEnum};

/// Pomodoro timer with white/pink/brown noise
#[derive(Parser, Debug)]
#[command(name = "pomonoise", version, about)]
struct Cli {
    /// Work duration (e.g., 25m, 1500s)
    #[arg(long, default_value = "25m", value_parser = parse_duration)]
    work: Duration,

    /// Break duration (e.g., 5m, 300s)
    #[arg(long, default_value = "5m", value_parser = parse_duration)]
    r#break: Duration,

    /// Number of work/break sets
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(1..))]
    sets: u32,

    /// Noise type: white, pink, or brown
    #[arg(long, value_enum, default_value_t = CliNoise::Pink)]
    noise: CliNoise,

    /// Volume level (0.0 to 1.0)
    #[arg(long, default_value_t = 0.5, value_parser = parse_volume)]
    volume: f32,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliNoise {
    White,
    Pink,
    Brown,
}

impl From<CliNoise> for app::NoiseType {
    fn from(value: CliNoise) -> Self {
        match value {
            CliNoise::White => app::NoiseType::White,
            CliNoise::Pink => app::NoiseType::Pink,
            CliNoise::Brown => app::NoiseType::Brown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AppConfig {
    work_duration: Duration,
    break_duration: Duration,
    sets: NonZeroU32,
    noise: app::NoiseType,
    volume: f32,
}

impl From<Cli> for AppConfig {
    fn from(cli: Cli) -> Self {
        Self {
            work_duration: cli.work,
            break_duration: cli.r#break,
            sets: NonZeroU32::new(cli.sets).expect("clap validated --sets to be >= 1"),
            noise: cli.noise.into(),
            volume: cli.volume,
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if let Some(mins) = s.strip_suffix('m') {
        let n: u64 = mins.parse().map_err(|_| format!("invalid duration: {s}"))?;
        let secs = n
            .checked_mul(60)
            .ok_or_else(|| format!("duration too large: {s}"))?;
        Ok(Duration::from_secs(secs))
    } else if let Some(secs) = s.strip_suffix('s') {
        let n: u64 = secs.parse().map_err(|_| format!("invalid duration: {s}"))?;
        Ok(Duration::from_secs(n))
    } else {
        Err(format!("duration must end with 'm' or 's': {s}"))
    }
}

fn parse_volume(s: &str) -> Result<f32, String> {
    let value: f32 = s.parse().map_err(|_| format!("invalid volume: {s}"))?;
    if !value.is_finite() {
        return Err(format!("volume must be finite: {s}"));
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(format!("volume out of range [0.0, 1.0]: {s}"));
    }
    Ok(value)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from(Cli::parse());

    let app_state = app::AppState::new(
        config.work_duration,
        config.break_duration,
        config.sets,
        config.noise,
        config.volume,
    );

    ui::run(app_state)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("25m").unwrap(), Duration::from_secs(25 * 60));
    }

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("300s").unwrap(), Duration::from_secs(300));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("25").is_err());
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn test_parse_duration_empty_string() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("  ").is_err());
    }

    #[test]
    fn test_parse_duration_zero() {
        assert_eq!(parse_duration("0m").unwrap(), Duration::from_secs(0));
        assert_eq!(parse_duration("0s").unwrap(), Duration::from_secs(0));
    }

    #[test]
    fn test_parse_duration_whitespace() {
        assert_eq!(
            parse_duration("  25m  ").unwrap(),
            Duration::from_secs(25 * 60)
        );
    }

    #[test]
    fn test_parse_duration_overflow() {
        // u64::MAX minutes would overflow when multiplied by 60
        let huge = format!("{}m", u64::MAX);
        assert!(parse_duration(&huge).is_err());
    }

    #[test]
    fn test_parse_duration_suffix_only() {
        assert!(parse_duration("m").is_err());
        assert!(parse_duration("s").is_err());
    }

    #[test]
    fn test_parse_volume_valid() {
        assert!((parse_volume("0.0").unwrap() - 0.0).abs() < f32::EPSILON);
        assert!((parse_volume("0.5").unwrap() - 0.5).abs() < f32::EPSILON);
        assert!((parse_volume("1.0").unwrap() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_volume_invalid() {
        assert!(parse_volume("-0.1").is_err());
        assert!(parse_volume("1.1").is_err());
        assert!(parse_volume("NaN").is_err());
        assert!(parse_volume("inf").is_err());
        assert!(parse_volume("").is_err());
    }

    #[test]
    fn test_cli_rejects_zero_sets() {
        assert!(Cli::try_parse_from(["pomonoise", "--sets", "0"]).is_err());
    }

    #[test]
    fn test_cli_rejects_non_finite_volume() {
        assert!(Cli::try_parse_from(["pomonoise", "--volume", "NaN"]).is_err());
    }

    #[test]
    fn test_cli_accepts_noise_enum_case_insensitive() {
        let cli = Cli::try_parse_from(["pomonoise", "--noise", "brown"]).unwrap();
        assert!(matches!(cli.noise, CliNoise::Brown));
    }
}

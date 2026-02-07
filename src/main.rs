mod app;
mod audio;
mod ui;

use clap::Parser;

/// Pomodoro timer with white/pink/brown noise
#[derive(Parser, Debug)]
#[command(name = "pomonoise", version, about)]
struct Cli {
    /// Work duration (e.g., 25m, 1500s)
    #[arg(long, default_value = "25m")]
    work: String,

    /// Break duration (e.g., 5m, 300s)
    #[arg(long, default_value = "5m")]
    r#break: String,

    /// Number of work/break sets
    #[arg(long, default_value_t = 4)]
    sets: u32,

    /// Noise type: white, pink, or brown
    #[arg(long, default_value = "pink")]
    noise: String,

    /// Volume level (0.0 to 1.0)
    #[arg(long, default_value_t = 0.5)]
    volume: f32,
}

fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
    let s = s.trim();
    if let Some(mins) = s.strip_suffix('m') {
        let n: u64 = mins.parse().map_err(|_| format!("invalid duration: {s}"))?;
        Ok(std::time::Duration::from_secs(n * 60))
    } else if let Some(secs) = s.strip_suffix('s') {
        let n: u64 = secs.parse().map_err(|_| format!("invalid duration: {s}"))?;
        Ok(std::time::Duration::from_secs(n))
    } else {
        Err(format!("duration must end with 'm' or 's': {s}"))
    }
}

fn parse_noise(s: &str) -> Result<app::NoiseType, String> {
    match s.to_lowercase().as_str() {
        "white" => Ok(app::NoiseType::White),
        "pink" => Ok(app::NoiseType::Pink),
        "brown" => Ok(app::NoiseType::Brown),
        _ => Err(format!(
            "unknown noise type: {s} (expected white, pink, or brown)"
        )),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let work_duration = parse_duration(&cli.work)?;
    let break_duration = parse_duration(&cli.r#break)?;
    let noise = parse_noise(&cli.noise)?;
    let volume = cli.volume.clamp(0.0, 1.0);

    let app_state = app::AppState::new(work_duration, break_duration, cli.sets, noise, volume);

    ui::run(app_state)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(
            parse_duration("25m").unwrap(),
            std::time::Duration::from_secs(25 * 60)
        );
    }

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(
            parse_duration("300s").unwrap(),
            std::time::Duration::from_secs(300)
        );
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("25").is_err());
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn test_parse_noise() {
        assert_eq!(parse_noise("white").unwrap(), app::NoiseType::White);
        assert_eq!(parse_noise("Pink").unwrap(), app::NoiseType::Pink);
        assert_eq!(parse_noise("BROWN").unwrap(), app::NoiseType::Brown);
        assert!(parse_noise("blue").is_err());
    }
}

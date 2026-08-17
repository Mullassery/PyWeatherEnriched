//! `pyweatherenriched-validate` — a small, real smoke-test CLI.
//!
//! Exercises the actual library end to end (forward geocoding via Nominatim,
//! then a real historical-weather fetch via Open-Meteo) so a maintainer or
//! CI job can confirm that a build/environment can actually reach the live
//! services PyWeatherEnriched depends on, without spinning up Python.
//!
//! Exit code is 0 on success, 1 on failure, so it's usable as a CI gate:
//!
//! ```text
//! $ cargo run --release --bin pyweatherenriched-validate -- --location Berlin
//! ```

use clap::Parser;
use pyweatherenriched::WeatherEnricher;

#[derive(Parser)]
#[command(
    name = "pyweatherenriched-validate",
    about = "Smoke-test PyWeatherEnriched's live geocoding + weather fetch against the real public APIs it depends on."
)]
struct Args {
    /// Location name to geocode (e.g. "New York" or "Paris, France").
    #[arg(long, default_value = "London")]
    location: String,

    /// Timestamp to fetch historical weather for.
    #[arg(long, default_value = "2024-06-15T12:00:00")]
    timestamp: String,

    /// In-memory cache capacity for the enricher used during validation.
    #[arg(long, default_value_t = 10)]
    cache_size: usize,
}

fn main() {
    let args = Args::parse();

    println!("pyweatherenriched-validate: checking live geocoding + weather fetch");
    println!("  location:  {}", args.location);
    println!("  timestamp: {}", args.timestamp);

    let enricher = WeatherEnricher::new(args.cache_size);

    match enricher.enrich(&args.location, &args.timestamp) {
        Ok(data) => {
            println!("OK: geocoding + weather fetch succeeded");
            println!("  latitude:    {:.4}", data.latitude);
            println!("  longitude:   {:.4}", data.longitude);
            println!("  temperature: {:.1} C", data.temperature);
            println!("  humidity:    {:.1}%", data.humidity);
            println!("  condition:   {}", data.condition);
        }
        Err(e) => {
            eprintln!("FAILED: {e}");
            std::process::exit(1);
        }
    }
}

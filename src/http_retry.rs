//! Retry-with-backoff for the Nominatim/Open-Meteo HTTP clients.
//!
//! Both `geocoder.rs` and `enricher.rs` previously called `.send()` once
//! with no retry at all: a transient network error or a 429 (rate limited)
//! response from either free public API failed the whole request
//! immediately. This retries on 429 and 5xx responses (and on transport-
//! level send errors) with exponential backoff plus jitter, honoring a
//! numeric `Retry-After` header on 429 when the server sends one.

use anyhow::{anyhow, Result};
use reqwest::blocking::{RequestBuilder, Response};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 500;

/// Send `request`, retrying on 429/5xx responses or transport errors with
/// exponential backoff (base 500ms, doubling each attempt) plus jitter, up
/// to `MAX_RETRIES` additional attempts (4 total). On the final attempt, a
/// 429/5xx response is returned as-is (letting the caller's existing
/// status-code handling surface it) rather than retried further.
pub fn send_with_retry(request: RequestBuilder) -> Result<Response> {
    let mut last_transport_err = None;

    for attempt in 0..=MAX_RETRIES {
        let builder = request
            .try_clone()
            .ok_or_else(|| anyhow!("request body is not cloneable, cannot retry"))?;

        match builder.send() {
            Ok(response) => {
                let status = response.status();
                let is_retryable = status.as_u16() == 429 || status.is_server_error();
                if !is_retryable || attempt == MAX_RETRIES {
                    return Ok(response);
                }
                let delay = retry_after_delay(&response).unwrap_or_else(|| backoff_delay(attempt));
                std::thread::sleep(delay);
            }
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(anyhow!(
                        "request failed after {} attempts: {e}",
                        MAX_RETRIES + 1
                    ));
                }
                last_transport_err = Some(e);
                std::thread::sleep(backoff_delay(attempt));
            }
        }
    }

    // Unreachable in practice (every loop iteration either returns or
    // sleeps-and-continues, and the last iteration always returns), but
    // keeps this a total function rather than relying on that invariant.
    Err(last_transport_err
        .map(|e| anyhow!("request failed: {e}"))
        .unwrap_or_else(|| anyhow!("request failed with no attempts recorded")))
}

fn backoff_delay(attempt: u32) -> Duration {
    let base = BASE_DELAY_MS.saturating_mul(1u64 << attempt);
    Duration::from_millis(base + jitter_ms(base))
}

/// A lightweight, non-cryptographic jitter source (0..=base/2 ms) so
/// concurrent callers backing off after a shared rate limit don't all
/// retry in lockstep. Not a real dependency worth adding `rand` for.
fn jitter_ms(base: u64) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let max_jitter = (base / 2).max(1);
    nanos % max_jitter
}

fn retry_after_delay(response: &Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_succeeds_immediately_on_200() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body("ok")
            .expect(1)
            .create();

        let client = reqwest::blocking::Client::new();
        let response = send_with_retry(client.get(server.url())).unwrap();

        assert_eq!(response.status(), 200);
        mock.assert();
    }

    #[test]
    fn test_retries_on_429_then_succeeds() {
        let mut server = mockito::Server::new();
        let rate_limited = server.mock("GET", "/").with_status(429).create();
        // mockito serves mocks in registration order for the same
        // matcher until one hits its expectation limit, so this first
        // request gets the 429 above, and the retry gets the 200 below.
        rate_limited.expect(1);
        let ok = server.mock("GET", "/").with_status(200).create();

        let client = reqwest::blocking::Client::new();
        let response = send_with_retry(client.get(server.url())).unwrap();

        assert_eq!(response.status(), 200);
        ok.assert();
    }

    #[test]
    fn test_gives_up_after_max_retries_on_persistent_5xx() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(503)
            .expect(1 + MAX_RETRIES as usize)
            .create();

        let client = reqwest::blocking::Client::new();
        let response = send_with_retry(client.get(server.url())).unwrap();

        // The final attempt's response is returned as-is, not an error --
        // callers' existing status-code handling surfaces it.
        assert_eq!(response.status(), 503);
        mock.assert();
    }

    #[test]
    fn test_does_not_retry_on_4xx_other_than_429() {
        let mut server = mockito::Server::new();
        let mock = server.mock("GET", "/").with_status(404).expect(1).create();

        let client = reqwest::blocking::Client::new();
        let response = send_with_retry(client.get(server.url())).unwrap();

        assert_eq!(response.status(), 404);
        mock.assert();
    }
}

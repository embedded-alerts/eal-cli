use anyhow::{bail, Context};
use flags2env::BundledFlags2Env;
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

fn apply_flags() -> anyhow::Result<String> {
    let parser = BundledFlags2Env::new();
    parser
        .audit_config(Some(".cli-flags.toml"))
        .map_err(|error| anyhow::anyhow!("invalid .cli-flags.toml: {error}"))?;

    let argv = std::env::args().collect::<Vec<_>>();
    let parsed = parser
        .parse_structured(&argv, Some(".cli-flags.toml"))
        .map_err(|error| anyhow::anyhow!("could not parse CLI arguments: {error}"))?;

    if !parsed.unknown_options.is_empty() || !parsed.errors.is_empty() {
        bail!(
            "invalid CLI arguments: unknown={:?}, errors={:?}",
            parsed.unknown_options,
            parsed.errors
        );
    }

    let command = parsed.command.clone();
    for (key, value) in parsed.provided_flags {
        // SAFETY: flags are applied before any worker task is spawned or any
        // environment-reading client is constructed.
        unsafe { std::env::set_var(key, value) };
    }

    Ok(command)
}

fn validate_base_url(value: &str) -> anyhow::Result<String> {
    let parsed = reqwest::Url::parse(value)
        .context("EAL_BASE_URL must be an absolute HTTP(S) URL")?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("EAL_BASE_URL must use HTTP(S) and contain no credentials, query, or fragment");
    }
    Ok(parsed.to_string().trim_end_matches('/').to_owned())
}


fn build_alert_url(base: &str, id: &str) -> anyhow::Result<reqwest::Url> {
    // IDs are single opaque path segments, not URL fragments or paths.
    if id.is_empty()
        || id.len() > 256
        || matches!(id, "." | "..")
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        bail!("EAL_ID must be a non-empty, bounded URL path identifier");
    }
    let mut url = reqwest::Url::parse(&format!("{base}/v1/alerts"))?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("EAL_BASE_URL must support path segments"))?
        .push(id);
    Ok(url)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = apply_flags()?;
    let base =
        std::env::var("EAL_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let base = validate_base_url(&base)?;
    let timeout = std::env::var("EAL_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(20);
    let output = std::env::var("EAL_OUTPUT").unwrap_or_else(|_| "json".into());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()?;

    match command.as_str() {
        "health" => {
            print_response(
                client.get(format!("{base}/healthz")).send().await?,
                &output,
            )
            .await
        }
        "list" => {
            print_response(
                client.get(format!("{base}/v1/alerts")).send().await?,
                &output,
            )
            .await
        }
        "get" => {
            let id = std::env::var("EAL_ID").context("--id is required")?;
            print_response(
                client
                    .get(build_alert_url(&base, &id)?)
                    .send()
                    .await?,
                &output,
            )
            .await
        }
        "watch" => watch(&base).await,
        _ => bail!("choose one command: health, list, get, watch"),
    }
}

async fn print_response(response: reqwest::Response, output: &str) -> anyhow::Result<()> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        bail!("HTTP {status}: {text}");
    }
    if output == "json" {
        let value: serde_json::Value = serde_json::from_str(&text)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{text}");
    }
    Ok(())
}

async fn watch(base: &str) -> anyhow::Result<()> {
    let ws_base = base
        .replacen("http://", "ws://", 1)
        .replacen("https://", "wss://", 1);
    let (socket, _) = connect_async(format!("{ws_base}/v1/ws")).await?;
    let (_, mut incoming) = socket.split();
    while let Some(message) = incoming.next().await {
        println!("{}", message?.into_text()?);
    }
    Ok(())
}

#[cfg(test)]
mod url_validation_tests {
    use super::{build_alert_url, validate_base_url};

    #[test]
    fn valid_http_and_https_bases_are_canonicalized() {
        assert_eq!(
            validate_base_url("http://127.0.0.1:8080/").unwrap(),
            "http://127.0.0.1:8080"
        );
        assert_eq!(
            validate_base_url("https://example.test/api/").unwrap(),
            "https://example.test/api"
        );
    }

    #[test]
    fn alert_identifiers_are_single_bounded_path_segments() {
        let base = validate_base_url("https://example.test/api/").unwrap();
        assert_eq!(
            build_alert_url(&base, "abc:12-foo_bar.9").unwrap().as_str(),
            "https://example.test/api/v1/alerts/abc:12-foo_bar.9"
        );
        for invalid in [
            "",
            ".",
            "..",
            "../admin",
            "a/b",
            "a?token=secret",
            "a#fragment",
            "%2fadmin",
            "a\\nother",
            "💡",
        ] {
            assert!(build_alert_url(&base, invalid).is_err(), "accepted unsafe ID: {invalid:?}");
        }
        assert!(build_alert_url(&base, &"x".repeat(257)).is_err());
        assert!(build_alert_url(&base, &"x".repeat(256)).is_ok());
    }

    #[test]
    fn invalid_and_credentialed_bases_are_rejected_before_network_io() {
        for value in [
            "",
            "/api",
            "file:///tmp/secret",
            "ftp://example.test",
            "http://user@example.test",
            "https://user:pass@example.test",
            "https://example.test/path?token=secret",
            "https://example.test/path#fragment",
        ] {
            assert!(validate_base_url(value).is_err(), "accepted unsafe URL: {value}");
        }
    }
}

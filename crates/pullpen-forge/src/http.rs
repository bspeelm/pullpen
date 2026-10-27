//! Requests, and the mapping from HTTP failures to `Error` that both
//! clients share (PLAN.md §4.2).

use reqwest::{RequestBuilder, Response, StatusCode};
use serde_json::Value;

use crate::{Error, Result};

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("pullpen/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        // Building fails only if the TLS backend cannot initialise, which no
        // retry fixes; the default client says the same thing later, per call.
        .unwrap_or_default()
}

/// The forge's own explanation from an error body: GitHub's `errors` (each a
/// string or an object with a `message`) in preference to its generic
/// `message`, GitLab's `message` (a string, or an object of lists) or `error`.
fn explanation(body: &Value) -> Option<String> {
    let specific: Vec<String> = body
        .get("errors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| e.as_str().or_else(|| e.get("message").and_then(Value::as_str)).map(String::from))
        .collect();
    // GitHub says "Unprocessable Entity" for every 422; the errors say why.
    if !specific.is_empty() {
        return Some(specific.join("; "));
    }
    let mut parts = Vec::new();
    match body.get("message") {
        Some(Value::String(m)) => parts.push(m.clone()),
        Some(Value::Object(o)) => {
            for (k, v) in o {
                let v = v
                    .as_array()
                    .map_or(v.to_string(), |a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "));
                parts.push(format!("{k} {v}"));
            }
        }
        Some(Value::Array(a)) => parts.extend(a.iter().filter_map(Value::as_str).map(String::from)),
        _ => {}
    }
    if let Some(e) = body.get("error").and_then(Value::as_str) {
        parts.push(e.to_string());
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

pub async fn send(host: &str, req: RequestBuilder) -> Result<Value> {
    let text = text(host, req).await?;
    Ok(if text.is_empty() { Value::Null } else { serde_json::from_str(&text).unwrap_or(Value::String(text)) })
}

/// The body as text, for a job's log: a log that happens to be valid JSON
/// must stay a log.
pub async fn text(host: &str, req: RequestBuilder) -> Result<String> {
    let resp = req.send().await.map_err(|e| Error::Network(format!("{host}: {}", without_url(&e))))?;
    check(host, resp).await
}

/// reqwest's errors print the request URL, and a URL can carry a token in
/// its query string; the host is enough to say where it failed.
fn without_url(e: &reqwest::Error) -> String {
    let mut msg = e.to_string();
    if let Some(u) = e.url() {
        msg = msg.replace(u.as_str(), "");
    }
    msg.trim().trim_end_matches(['(', ')', ' ', ':']).to_string()
}

async fn check(host: &str, resp: Response) -> Result<String> {
    let status = resp.status();
    let retry_after = header_secs(&resp, "retry-after");
    let reset = header_secs(&resp, "x-ratelimit-reset");
    let exhausted = resp.headers().get("x-ratelimit-remaining").is_some_and(|v| v == "0");
    let text = resp.text().await.map_err(|e| Error::Network(format!("{host}: {}", without_url(&e))))?;
    if status.is_success() {
        return Ok(text);
    }
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    let why = explanation(&body);
    Err(match status {
        StatusCode::UNAUTHORIZED => Error::Auth(host.into()),
        StatusCode::TOO_MANY_REQUESTS => Error::RateLimited { retry_in: retry_after.unwrap_or(60) },
        StatusCode::FORBIDDEN if exhausted || retry_after.is_some() => {
            let now = pullpen_core::time::Timestamp::now().0 as u64;
            Error::RateLimited { retry_in: retry_after.or(reset.map(|r| r.saturating_sub(now))).unwrap_or(60).max(1) }
        }
        s if s.is_server_error() => Error::Server { host: host.into(), status: s.as_u16() },
        s => Error::Refused(why.unwrap_or_else(|| format!("{host} answered {}", s.as_u16()))),
    })
}

fn header_secs(r: &Response, name: &str) -> Option<u64> {
    r.headers().get(name)?.to_str().ok()?.parse().ok()
}

/// A GraphQL call. A mutation that fails still answers with `data`, its one
/// field null, so success means at least one non-null field; otherwise the
/// first error's message becomes the refusal.
pub async fn graphql(host: &str, req: RequestBuilder, query: &str, vars: Value) -> Result<Value> {
    graphql_partial(host, req, query, vars).await.map(|(d, _)| d)
}

/// As `graphql`, and also why part of the answer is missing, when it is:
/// a search can come back with some rows withheld and the rest intact.
pub async fn graphql_partial(
    host: &str,
    req: RequestBuilder,
    query: &str,
    vars: Value,
) -> Result<(Value, Option<String>)> {
    let body = send(host, req.json(&serde_json::json!({ "query": query, "variables": vars }))).await?;
    let data = body.get("data").filter(|d| d.as_object().is_some_and(|o| o.values().any(|v| !v.is_null())));
    match (data, explanation(&body)) {
        (Some(d), why) => {
            if let Some(why) = &why {
                tracing::warn!(%host, %why, "partial GraphQL answer");
            }
            // One error per withheld row says the same thing many times.
            let why = why.map(|w| {
                let mut seen: Vec<&str> = Vec::new();
                w.split("; ")
                    .filter(|p| {
                        !seen.contains(p) && {
                            seen.push(p);
                            true
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            });
            Ok((d.clone(), why))
        }
        (None, Some(why)) => Err(Error::Refused(why)),
        // Every field null and no error: GitLab's answer for a group or
        // project that does not exist. The caller knows what it asked for,
        // so it says what was not found.
        (None, None) => Ok((body.get("data").cloned().unwrap_or(Value::Null), None)),
    }
}

/// Percent-encodes a GitLab project path for use as a REST `:id`.
pub fn encode_path(p: &str) -> String {
    p.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

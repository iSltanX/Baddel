//! Problem reports: the one network path besides the updater, and only ever taken after the
//! user has seen the exact payload and confirmed it.
//!
//! Reports go to the shared Worker of `iSltanX/app-reports`, which files them as issues in a
//! private repository (contract v1, in that repository's README). The app holds no secret. The
//! payload is built here and nowhere else, so the preview, the copy and the request are the same
//! bytes; it carries the user's description and chosen image, the diagnostics in
//! [`crate::diagnostics`], and nothing typed into or converted in any other app.

use std::sync::{Mutex, Once};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::diagnostics::Snapshot;

/// The shared Worker. Its host is fixed by the first deploy (see `server/report-worker/`).
pub const ENDPOINT: &str = "https://app-reports.isultantf.workers.dev/v1/reports";
/// The product id registered in the shared repository (`product:baddel`).
pub const PRODUCT: &str = "baddel";
/// Stricter than the contract's 2,000: a report is a short description, not a document.
pub const MAX_DESCRIPTION: usize = 1000;
const TIMEOUT: Duration = Duration::from_secs(20);

/// What the user picks from the problem-type list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Problem {
    WrongConversion,
    NoEffect,
    Undo,
    Shortcut,
    Crash,
    Suggestion,
    Other,
}

impl Problem {
    /// The contract's `kind`, which becomes the issue's `kind:*` label.
    pub fn kind(self) -> &'static str {
        match self {
            Problem::WrongConversion | Problem::NoEffect | Problem::Undo | Problem::Shortcut => "bug",
            Problem::Crash => "crash",
            Problem::Suggestion => "suggestion",
            Problem::Other => "other",
        }
    }

    /// Baddel's own sub-type, shown in the issue.
    pub fn category(self) -> Option<&'static str> {
        match self {
            Problem::WrongConversion => Some("conversion"),
            Problem::NoEffect => Some("no-effect"),
            Problem::Undo => Some("undo"),
            Problem::Shortcut => Some("shortcut"),
            Problem::Crash | Problem::Suggestion | Problem::Other => None,
        }
    }
}

/// What the report window sends for a preview or a send.
#[derive(Clone, Debug, Deserialize)]
pub struct Draft {
    pub problem: Problem,
    pub description: String,
}

/// The prepared attachment (see `sys::image`).
pub struct Image {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub thumbnail: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DraftError {
    EmptyDescription,
    DescriptionTooLong,
}

/// The report as it goes over the wire.
pub fn payload(draft: &Draft, snapshot: &Snapshot, image: Option<&Image>) -> Result<Value, DraftError> {
    let description = draft.description.trim();
    match description.chars().count() {
        0 => return Err(DraftError::EmptyDescription),
        n if n > MAX_DESCRIPTION => return Err(DraftError::DescriptionTooLong),
        _ => {}
    }
    let e = &snapshot.envelope;
    let mut report = json!({
        "product": PRODUCT,
        "app_version": e.app_version,
        "os": e.os,
        "os_version": e.os_version,
        "arch": e.arch,
        "locale": e.locale,
        "kind": draft.problem.kind(),
        "description": description,
        "diagnostics": snapshot.details,
    });
    let fields = report.as_object_mut().expect("an object literal");
    if let Some(category) = draft.problem.category() {
        fields.insert("category".into(), category.into());
    }
    if let Some(image) = image {
        fields.insert("attachments".into(), json!([{ "type": image.mime, "data": base64(&image.bytes) }]));
    }
    // Development builds mark their reports, so they never pass for a user's.
    if cfg!(debug_assertions) {
        fields.insert("test".into(), true.into());
    }
    Ok(report)
}

/// The payload as the preview shows it and "Copy Report" copies it: identical, except that each
/// attachment's data is replaced by its type and size (the contract's copy format).
pub fn for_display(payload: &Value) -> Value {
    let mut shown = payload.clone();
    if let Some(attachments) = shown.get_mut("attachments").and_then(Value::as_array_mut) {
        for attachment in attachments {
            let bytes = attachment.get("data").and_then(Value::as_str).map_or(0, decoded_len);
            let mime = attachment.get("type").cloned().unwrap_or(Value::Null);
            *attachment = json!({ "type": mime, "bytes": bytes });
        }
    }
    shown
}

pub fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

// ── Sending ──────────────────────────────────────────────────────────────────

/// Why a send did not produce a report number, and so what the window offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SendError {
    /// Offline, timed out, or a server error: Retry and Copy Report.
    Retry,
    /// Too many reports from this network: Retry later and Copy Report.
    RateLimited { minutes: u64 },
    /// The server refused this report as it is: Copy Report only.
    Rejected { reason: String },
}

/// Posts the report. `key` is the report's idempotency key, reused on its retries.
pub async fn send(payload: &Value, key: &str, version: &str) -> Result<u64, SendError> {
    install_crypto_provider();
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(format!("Baddel/{version}"))
        .build()
        .map_err(|_| SendError::Retry)?;
    let response = client
        .post(endpoint())
        .header("Idempotency-Key", key)
        .json(payload)
        .send()
        .await
        .map_err(|_| SendError::Retry)?;
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let body: Value = response.json().await.unwrap_or(Value::Null);
    interpret(status, retry_after, &body)
}

/// Maps the Worker's answer to an outcome (contract v1 → "Responses").
fn interpret(status: u16, retry_after: Option<u64>, body: &Value) -> Result<u64, SendError> {
    match status {
        200 | 201 => body.get("id").and_then(Value::as_u64).ok_or(SendError::Retry),
        429 => Err(SendError::RateLimited { minutes: retry_after.map_or(60, |s| s.div_ceil(60).max(1)) }),
        400 | 413 | 415 => {
            let error = body.get("error").and_then(Value::as_str).unwrap_or("rejected");
            let reason = match body.get("field").and_then(Value::as_str) {
                Some(field) => format!("{error}: {field}"),
                None => error.to_string(),
            };
            Err(SendError::Rejected { reason })
        }
        _ => Err(SendError::Retry),
    }
}

fn endpoint() -> String {
    // Debug aid for the failure path: point a development build at an address that does not answer.
    #[cfg(debug_assertions)]
    if let Ok(url) = std::env::var("BADDEL_REPORT_ENDPOINT") {
        return url;
    }
    ENDPOINT.to_string()
}

/// reqwest is built without a bundled TLS crypto provider (the updater's configuration, shared
/// so no second TLS stack enters the build); the process-wide default is installed once here.
fn install_crypto_provider() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
    });
}

// ── The window's session ─────────────────────────────────────────────────────

/// What the report window has chosen so far that must not travel through the webview: the
/// prepared image, and the idempotency key of the report last sent.
#[derive(Default)]
pub struct Session {
    pub image: Option<Image>,
    /// The payload the preview showed. Send and Copy Report use this very value, so what goes
    /// out is what the user saw, down to the diagnostics' timings.
    pub previewed: Option<Value>,
    sent: Option<(String, String)>,
}

impl Session {
    /// The key for this exact payload: the same one on a retry, a new one once anything changed.
    pub fn key_for(&mut self, payload: &str) -> String {
        match &self.sent {
            Some((last, key)) if last == payload => key.clone(),
            _ => {
                let key = uuid_v4();
                self.sent = Some((payload.to_string(), key.clone()));
                key
            }
        }
    }

    pub fn clear(&mut self) {
        *self = Session::default();
    }
}

#[derive(Default)]
pub struct ReportState(pub Mutex<Session>);

/// A random UUID (version 4). Made per report, never stored, never tied to the user or the Mac.
fn uuid_v4() -> String {
    use std::io::Read;
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_err() {
        // Not expected on macOS; a time-based fallback still differs per report.
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        b = nanos.to_le_bytes();
    }
    b[6] = (b[6] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

// ── Base64 ───────────────────────────────────────────────────────────────────

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding, as the contract's `data` field expects.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn decoded_len(encoded: &str) -> usize {
    let padding = encoded.bytes().rev().take_while(|&b| b == b'=').count();
    encoded.len() / 4 * 3 - padding
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::Outcome;
    use crate::diagnostics::{Details, Envelope, LayoutChoice, Layouts, Path, Recent, SettingsSummary, Shortcuts};
    use crate::settings::Language;

    fn snapshot() -> Snapshot {
        Snapshot {
            envelope: Envelope {
                app_version: "1.1.0".into(),
                os: "macos",
                os_version: "27.0.1".into(),
                arch: "arm64",
                locale: "ar",
            },
            details: Details {
                accessibility: true,
                secure_input: false,
                layouts: Layouts {
                    arabic: Some("com.apple.keylayout.Arabic-PC".into()),
                    arabic_name: Some("Arabic – PC".into()),
                    latin: Some("com.apple.keylayout.ABC".into()),
                    latin_name: Some("ABC".into()),
                    active: "latin",
                    source: "live",
                },
                layout_choice: LayoutChoice { arabic: "auto".into(), latin: "auto".into() },
                recent: vec![Recent { outcome: Outcome::Converted, path: Some(Path::Keys), ms: 1240, ago_s: 42 }],
                last_app: Some("com.apple.mail".into()),
                settings: SettingsSummary {
                    language: Language::Ar,
                    launch_at_login: false,
                    show_tray_icon: true,
                    switch_input_source: true,
                    show_hud: true,
                    sound: false,
                    auto_update: true,
                    paused: false,
                    shortcuts: Shortcuts { convert: "Alt+Shift+Space".into(), undo: String::new(), pause: String::new() },
                    excluded_count: 14,
                    excluded_default: true,
                },
                build: "debug",
            },
        }
    }

    fn draft(description: &str) -> Draft {
        Draft { problem: Problem::WrongConversion, description: description.into() }
    }

    const CONTRACT_FIELDS: [&str; 12] = [
        "product", "app_version", "os", "os_version", "arch", "locale", "kind", "category", "description",
        "diagnostics", "attachments", "test",
    ];

    #[test]
    fn payload_follows_the_contract() {
        let image = Image { mime: "image/png", bytes: vec![1, 2, 3, 4], width: 2, height: 2, thumbnail: vec![] };
        let p = payload(&draft("  فتحوّلت الكلمات الأولى فقط  "), &snapshot(), Some(&image)).unwrap();
        assert_eq!(p["product"], "baddel");
        assert_eq!(p["kind"], "bug");
        assert_eq!(p["category"], "conversion");
        assert_eq!(p["description"], "فتحوّلت الكلمات الأولى فقط");
        assert_eq!(p["attachments"][0]["type"], "image/png");
        assert_eq!(p["attachments"][0]["data"], "AQIDBA==");
        assert_eq!(p["test"], cfg!(debug_assertions));
        // Unknown top-level fields are rejected by the Worker: send only the contract's.
        for key in p.as_object().unwrap().keys() {
            assert!(CONTRACT_FIELDS.contains(&key.as_str()), "unexpected field {key}");
        }
    }

    #[test]
    fn kinds_without_a_category_leave_it_out() {
        let d = Draft { problem: Problem::Suggestion, description: "x".into() };
        let p = payload(&d, &snapshot(), None).unwrap();
        assert_eq!(p["kind"], "suggestion");
        assert!(p.get("category").is_none());
        assert!(p.get("attachments").is_none());
    }

    #[test]
    fn description_limits() {
        assert_eq!(payload(&draft("   "), &snapshot(), None), Err(DraftError::EmptyDescription));
        let long = "ب".repeat(MAX_DESCRIPTION + 1);
        assert_eq!(payload(&draft(&long), &snapshot(), None), Err(DraftError::DescriptionTooLong));
        assert!(payload(&draft(&"ب".repeat(MAX_DESCRIPTION)), &snapshot(), None).is_ok());
    }

    /// The privacy promise, as a test: apart from the description the user wrote, nothing in the
    /// payload is free text — and in particular nothing a conversion handled.
    #[test]
    fn payload_carries_no_conversion_text() {
        let p = payload(&draft("وصف"), &snapshot(), None).unwrap();
        let mut rest = p.clone();
        rest.as_object_mut().unwrap().remove("description");
        let serialized = rest.to_string();
        // The HUD, the menu and undo hold the last conversion; none of it can reach a report,
        // because `diagnostics::Details` has no field that could hold text.
        for text in ["اثممخ", "hello", "sghl", "سلام"] {
            assert!(!serialized.contains(text), "{text} leaked into the payload");
        }
        let diagnostics = p["diagnostics"].as_object().unwrap();
        let mut keys: Vec<&str> = diagnostics.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["accessibility", "build", "last_app", "layout_choice", "layouts", "recent", "secure_input", "settings"]
        );
        let recent = diagnostics["recent"][0].as_object().unwrap();
        let mut recent_keys: Vec<&str> = recent.keys().map(String::as_str).collect();
        recent_keys.sort_unstable();
        assert_eq!(recent_keys, ["ago_s", "ms", "outcome", "path"]);
    }

    #[test]
    fn display_replaces_image_data_with_its_size() {
        let image = Image { mime: "image/jpeg", bytes: vec![0; 1000], width: 10, height: 10, thumbnail: vec![] };
        let p = payload(&draft("x"), &snapshot(), Some(&image)).unwrap();
        let shown = for_display(&p);
        assert_eq!(shown["attachments"][0], json!({ "type": "image/jpeg", "bytes": 1000 }));
        assert_eq!(shown["description"], p["description"]);
    }

    #[test]
    fn responses_map_to_what_the_window_offers() {
        assert_eq!(interpret(201, None, &json!({ "id": 12 })), Ok(12));
        assert_eq!(interpret(200, None, &json!({ "id": 12 })), Ok(12));
        assert_eq!(interpret(201, None, &json!({})), Err(SendError::Retry));
        assert_eq!(interpret(429, Some(125), &Value::Null), Err(SendError::RateLimited { minutes: 3 }));
        assert_eq!(
            interpret(400, None, &json!({ "error": "invalid_field", "field": "description" })),
            Err(SendError::Rejected { reason: "invalid_field: description".into() })
        );
        assert_eq!(interpret(413, None, &json!({ "error": "too_large" })), Err(SendError::Rejected { reason: "too_large".into() }));
        assert_eq!(interpret(502, None, &Value::Null), Err(SendError::Retry));
    }

    #[test]
    fn idempotency_key_is_reused_only_for_the_same_payload() {
        let mut session = Session::default();
        let first = session.key_for("a");
        assert_eq!(session.key_for("a"), first);
        let second = session.key_for("b");
        assert_ne!(second, first);
        assert_eq!(second.len(), 36);
        assert_eq!(&second[14..15], "4");
    }

    /// The channel end to end, from this module's own payload and sender: files a real `test`
    /// report with an image. Opt-in and never part of `check.sh`:
    /// `BADDEL_LIVE_REPORT=1 cargo test -p baddel live_report -- --ignored --nocapture`
    #[test]
    #[ignore = "posts to the live Worker"]
    fn live_report() {
        if std::env::var_os("BADDEL_LIVE_REPORT").is_none() {
            return;
        }
        let png = crate::sys::image::tests::png(640.0, 400.0);
        let prepared = crate::sys::image::prepare_file(&png).unwrap();
        let image = Image { mime: prepared.mime, bytes: prepared.bytes, width: prepared.width, height: prepared.height, thumbnail: prepared.thumbnail };
        let d = Draft {
            problem: Problem::Other,
            description: "Channel check from src-tauri/src/report.rs (cargo test, not the report window). Safe to close.".into(),
        };
        let p = payload(&d, &snapshot(), Some(&image)).unwrap();
        let id = tauri::async_runtime::block_on(send(&p, &uuid_v4(), "test")).expect("the Worker filed the report");
        println!("filed report #{id}");
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(decoded_len(&base64(&[7; 1001])), 1001);
    }
}

//! Explicit, one-shot feedback submission. No feedback is queued or sent in the background.

use serde::Deserialize;
use serde_json::json;

use crate::assistant::transport::{
    HttpTransport, Method, Request, ReqwestTransport, TransportError,
};

const FEEDBACK_URL: &str = "https://buttonscli.com/bcli-metrics/api/v1/feedback";
const MAX_MESSAGE_CHARS: usize = 2_000;
const MAX_CONTACT_CHARS: usize = 160;
const HTTP: ReqwestTransport = ReqwestTransport;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedbackCategory {
    FeatureRequest,
    BugReport,
    UiIdea,
    AiHelp,
    Performance,
    Pricing,
    Other,
}

impl FeedbackCategory {
    pub(crate) const ALL: [Self; 7] = [
        Self::FeatureRequest,
        Self::BugReport,
        Self::UiIdea,
        Self::AiHelp,
        Self::Performance,
        Self::Pricing,
        Self::Other,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::FeatureRequest => "feature-request",
            Self::BugReport => "bug-report",
            Self::UiIdea => "ui-idea",
            Self::AiHelp => "ai-help",
            Self::Performance => "performance",
            Self::Pricing => "pricing",
            Self::Other => "other",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::FeatureRequest => "Feature request",
            Self::BugReport => "Bug report",
            Self::UiIdea => "UI idea",
            Self::AiHelp => "AI Help",
            Self::Performance => "Performance issue",
            Self::Pricing => "Pricing concern",
            Self::Other => "Other complaint or idea",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FeedbackSubmission {
    pub(crate) category: FeedbackCategory,
    pub(crate) message: String,
    pub(crate) contact: String,
    pub(crate) locale: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FeedbackReceipt {
    pub(crate) id: String,
}

#[derive(Debug)]
pub(crate) struct FeedbackEvent {
    pub(crate) generation: u64,
    pub(crate) result: Result<FeedbackReceipt, FeedbackError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedbackError {
    Disabled,
    Endpoint,
    Entropy,
    EmptyMessage,
    InvalidText,
    Network,
    Http(u16),
    ResponseTooLarge,
    InvalidReply,
}

impl std::fmt::Display for FeedbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Disabled => "feedback is disabled pending the native service",
            Self::Endpoint => "feedback endpoint is invalid",
            Self::Entropy => "could not create temporary feedback identifiers",
            Self::EmptyMessage => "feedback message is empty",
            Self::InvalidText => "feedback contains unsupported control characters",
            Self::Network => "could not reach the feedback service",
            Self::Http(_) => "feedback service rejected the submission",
            Self::ResponseTooLarge => "feedback response exceeded 1 MiB",
            Self::InvalidReply => "feedback service returned an invalid reply",
        })
    }
}

pub(crate) fn redacted_message(input: &str) -> String {
    let bounded: String = input.trim().chars().take(MAX_MESSAGE_CHARS).collect();
    let redacted = crate::session::context::redact_obvious_secrets(&bounded, None);
    redacted.chars().take(MAX_MESSAGE_CHARS).collect()
}

pub(crate) fn submit_feedback(
    submission: &FeedbackSubmission,
) -> Result<FeedbackReceipt, FeedbackError> {
    if !crate::account::LEGACY_METRICS_ENABLED {
        return Err(FeedbackError::Disabled);
    }
    submit_feedback_with(&HTTP, FEEDBACK_URL, submission)
}

fn submit_feedback_with(
    transport: &dyn HttpTransport,
    endpoint: &str,
    submission: &FeedbackSubmission,
) -> Result<FeedbackReceipt, FeedbackError> {
    let endpoint = validate_endpoint(endpoint)?;
    let message = redacted_message(&submission.message);
    if message.trim().is_empty() {
        return Err(FeedbackError::EmptyMessage);
    }
    let contact: String = submission
        .contact
        .trim()
        .chars()
        .take(MAX_CONTACT_CHARS)
        .collect();
    if !valid_feedback_text(&message) || !valid_feedback_text(&contact) {
        return Err(FeedbackError::InvalidText);
    }
    if submission.locale.len() > 32 || submission.locale.chars().any(char::is_control) {
        return Err(FeedbackError::InvalidText);
    }
    let install_id = temporary_id()?;
    let session_id = temporary_id()?;
    let body = json!({
        "installId": install_id,
        "sessionId": session_id,
        "category": submission.category.as_str(),
        "message": message,
        "contact": contact,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "meta": {
            "os": std::env::consts::OS,
            "locale": submission.locale,
        },
    });
    let response = transport
        .execute(Request {
            method: Method::Post,
            url: endpoint.to_string(),
            body: Some(body),
            key: None,
        })
        .map_err(|error| match error {
            TransportError::InvalidEndpoint => FeedbackError::Endpoint,
            TransportError::ResponseTooLarge => FeedbackError::ResponseTooLarge,
            TransportError::Network
            | TransportError::InvalidCredential
            | TransportError::Cancelled => FeedbackError::Network,
        })?;
    if !(200..300).contains(&response.status) {
        return Err(FeedbackError::Http(response.status));
    }
    let reply: FeedbackReply =
        serde_json::from_slice(&response.body).map_err(|_| FeedbackError::InvalidReply)?;
    if !reply.ok || reply.id.trim().is_empty() || reply.id.len() > 80 {
        return Err(FeedbackError::InvalidReply);
    }
    Ok(FeedbackReceipt { id: reply.id })
}

fn validate_endpoint(value: &str) -> Result<url::Url, FeedbackError> {
    let url = url::Url::parse(value).map_err(|_| FeedbackError::Endpoint)?;
    if url.scheme() != "https"
        || url.host_str() != Some("buttonscli.com")
        || url.port().is_some()
        || url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(FeedbackError::Endpoint);
    }
    let expected_path = "/bcli-metrics/api/v1/feedback";
    if url.path() != expected_path {
        return Err(FeedbackError::Endpoint);
    }
    Ok(url)
}

fn valid_feedback_text(value: &str) -> bool {
    !value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
}

fn temporary_id() -> Result<String, FeedbackError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| FeedbackError::Entropy)?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut id = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        id.push(HEX[(byte >> 4) as usize] as char);
        id.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FeedbackReply {
    ok: bool,
    id: String,
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::Value;

    use super::*;
    use crate::assistant::transport::{Response, TransportError};

    const TEST_ENDPOINT: &str = "https://buttonscli.com/bcli-metrics/api/v1/feedback";

    #[test]
    fn old_metrics_feedback_is_disabled_before_any_network_request() {
        let submission = FeedbackSubmission {
            category: FeedbackCategory::BugReport,
            message: "Synthetic feedback".into(),
            contact: String::new(),
            locale: "en".into(),
        };
        assert_eq!(submit_feedback(&submission), Err(FeedbackError::Disabled));
    }

    #[derive(Default)]
    struct MockTransport {
        status: u16,
        body: Vec<u8>,
        request: Mutex<Option<Request>>,
    }

    impl MockTransport {
        fn reply(status: u16, body: &str) -> Self {
            Self {
                status,
                body: body.as_bytes().to_vec(),
                request: Mutex::new(None),
            }
        }
    }

    impl HttpTransport for MockTransport {
        fn execute(&self, request: Request) -> Result<Response, TransportError> {
            *self.request.lock().unwrap() = Some(request);
            Ok(Response {
                status: self.status,
                body: self.body.clone(),
                content_type: Some("application/json".into()),
            })
        }
    }

    fn submission() -> FeedbackSubmission {
        FeedbackSubmission {
            category: FeedbackCategory::BugReport,
            message: "The app broke. api_key=hidden-token".into(),
            contact: "person@example.test".into(),
            locale: "en".into(),
        }
    }

    fn captured_request(transport: &MockTransport) -> Request {
        transport
            .request
            .lock()
            .unwrap()
            .take()
            .expect("feedback request was sent")
    }

    #[test]
    fn request_matches_the_audited_original_receiver_contract_and_sends_no_terminal_data() {
        let contract: Value =
            serde_json::from_str(include_str!("../tests/fixtures/feedback-contract.json")).unwrap();
        let transport = MockTransport::reply(200, r#"{"ok":true,"id":"abc123"}"#);
        let receipt = submit_feedback_with(&transport, TEST_ENDPOINT, &submission()).unwrap();
        assert_eq!(receipt.id, "abc123");

        let request = captured_request(&transport);
        assert!(matches!(request.method, Method::Post));
        assert_eq!(request.url, TEST_ENDPOINT);
        assert!(request.key.is_none());
        let body = request.body.unwrap();
        let fields: Vec<_> = body
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut required: Vec<_> = contract["requiredRequestFields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|field| field.as_str().unwrap())
            .collect();
        required.extend(["contact", "meta"]);
        let mut actual = fields;
        required.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, required);
        assert_eq!(body["category"], "bug-report");
        assert_eq!(body["message"], "The app broke. api_key=[REDACTED]");
        assert_eq!(body["contact"], "person@example.test");
        assert_eq!(body["meta"]["locale"], "en");
        assert_eq!(body["meta"]["os"], std::env::consts::OS);
        assert!(body["installId"].as_str().unwrap().len() <= 80);
        assert!(body["sessionId"].as_str().unwrap().len() <= 80);
        assert!(!body.to_string().contains("terminal"));
        assert!(!body.to_string().contains("clipboard"));
        assert!(!body.to_string().contains("filePath"));
    }

    #[test]
    fn non_success_status_and_invalid_success_body_are_never_reported_as_sent() {
        for status in [400, 429, 500] {
            let transport = MockTransport::reply(status, r#"{"ok":false,"error":"private"}"#);
            assert_eq!(
                submit_feedback_with(&transport, TEST_ENDPOINT, &submission()),
                Err(FeedbackError::Http(status))
            );
        }
        let transport = MockTransport::reply(200, r#"{"ok":false,"id":""}"#);
        assert_eq!(
            submit_feedback_with(&transport, TEST_ENDPOINT, &submission()),
            Err(FeedbackError::InvalidReply)
        );
        assert!(!transport
            .request
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .body
            .as_ref()
            .unwrap()
            .to_string()
            .contains("private"));
    }

    #[test]
    fn endpoint_and_input_bounds_fail_before_network_access() {
        let transport = MockTransport::reply(200, r#"{"ok":true,"id":"abc123"}"#);
        for endpoint in [
            "http://buttonscli.com/bcli-metrics/api/v1/feedback",
            "https://evil.example/bcli-metrics/api/v1/feedback",
            "https://buttonscli.com/other",
            "https://user@buttonscli.com/bcli-metrics/api/v1/feedback",
        ] {
            assert_eq!(
                submit_feedback_with(&transport, endpoint, &submission()),
                Err(FeedbackError::Endpoint)
            );
        }
        let mut empty = submission();
        empty.message = "  \n".into();
        assert_eq!(
            submit_feedback_with(&transport, TEST_ENDPOINT, &empty),
            Err(FeedbackError::EmptyMessage)
        );
        let mut control = submission();
        control.message = "invalid\u{7}".into();
        assert_eq!(
            submit_feedback_with(&transport, TEST_ENDPOINT, &control),
            Err(FeedbackError::InvalidText)
        );
        assert!(transport.request.lock().unwrap().is_none());
    }

    #[test]
    fn category_values_and_best_effort_redaction_match_the_contract_fixture() {
        let contract: Value =
            serde_json::from_str(include_str!("../tests/fixtures/feedback-contract.json")).unwrap();
        let allowed: Vec<_> = contract["categoryValues"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
        for category in FeedbackCategory::ALL {
            assert!(allowed.contains(&category.as_str()));
        }
        assert_eq!(
            redacted_message("Issue: Bearer super-secret-token\napi_key=hidden"),
            "Issue: Bearer [REDACTED]\napi_key=[REDACTED]"
        );
        assert_eq!(redacted_message(&"x".repeat(2_100)).chars().count(), 2_000);
        assert_eq!(submission().category.label(), "Bug report");
    }
}

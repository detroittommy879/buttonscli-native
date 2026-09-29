//! Bounded, read-only native guide documents.

use std::io::Read;
use std::time::Duration;

pub const MAX_GUIDE_URL_LENGTH: usize = 2_048;
pub const MAX_GUIDE_CONTENT_BYTES: usize = 256 * 1024;
pub const MAX_GUIDE_LINES: usize = 8_192;
pub const ONLINE_GUIDE_URL: &str = "https://buttonscli.com/dsp/test.md";
const ONLINE_GUIDE_HOSTS: [&str; 2] = ["buttonscli.com", "www.buttonscli.com"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuideTab {
    QuickStart,
    AiHelp,
    Online,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownBlock {
    Heading { level: u8, text: String },
    Bullet(String),
    Code(String),
    Paragraph(String),
    Spacer,
    Truncated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuideError {
    Endpoint,
    Network,
    HttpStatus,
    TooLarge,
    InvalidText,
}

#[derive(Debug)]
pub struct GuideEvent {
    pub generation: u64,
    pub result: Result<String, GuideError>,
}

pub fn bundled_body(tab: GuideTab) -> Option<&'static str> {
    match tab {
        GuideTab::QuickStart => Some(include_str!("../docs/NATIVE-GUIDE.md")),
        GuideTab::AiHelp => Some(include_str!("../docs/AI-HELP.md")),
        GuideTab::Online => None,
    }
}

pub fn validate_online_guide_url(raw: &str) -> bool {
    if raw.len() > MAX_GUIDE_URL_LENGTH {
        return false;
    }
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.port().is_none()
        && ONLINE_GUIDE_HOSTS.contains(&url.host_str().unwrap_or_default())
        && url.path().starts_with("/dsp/")
}

pub fn validate_external_url(raw: &str) -> bool {
    if raw.len() > MAX_GUIDE_URL_LENGTH {
        return false;
    }
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && ONLINE_GUIDE_HOSTS.contains(&url.host_str().unwrap_or_default())
}

pub fn validate_markdown(body: String) -> Result<String, GuideError> {
    if body.len() > MAX_GUIDE_CONTENT_BYTES {
        return Err(GuideError::TooLarge);
    }
    if body.trim().is_empty()
        || body
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(GuideError::InvalidText);
    }
    Ok(body.replace("\r\n", "\n").replace('\r', "\n"))
}

pub fn markdown_blocks(source: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut in_code = false;
    for line in source.lines().take(MAX_GUIDE_LINES) {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            blocks.push(MarkdownBlock::Code(line.to_owned()));
        } else if trimmed.is_empty() {
            blocks.push(MarkdownBlock::Spacer);
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            blocks.push(MarkdownBlock::Heading {
                level: 3,
                text: rest.to_owned(),
            });
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            blocks.push(MarkdownBlock::Heading {
                level: 2,
                text: rest.to_owned(),
            });
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            blocks.push(MarkdownBlock::Heading {
                level: 1,
                text: rest.to_owned(),
            });
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            blocks.push(MarkdownBlock::Bullet(rest.to_owned()));
        } else {
            blocks.push(MarkdownBlock::Paragraph(trimmed.to_owned()));
        }
    }
    if source.lines().nth(MAX_GUIDE_LINES).is_some() {
        blocks.push(MarkdownBlock::Truncated);
    }
    blocks
}

pub fn fetch_online_guide() -> Result<String, GuideError> {
    if !validate_online_guide_url(ONLINE_GUIDE_URL) {
        return Err(GuideError::Endpoint);
    }
    let client = guide_http_client(false)?;
    fetch_markdown_with_client(&client, ONLINE_GUIDE_URL)
}

fn guide_http_client(no_proxy: bool) -> Result<reqwest::blocking::Client, GuideError> {
    let mut builder = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none());
    if no_proxy {
        builder = builder.no_proxy();
    }
    builder.build().map_err(|_| GuideError::Network)
}

fn fetch_markdown_with_client(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<String, GuideError> {
    let mut response = client.get(url).send().map_err(|_| GuideError::Network)?;
    if !response.status().is_success() {
        return Err(GuideError::HttpStatus);
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !(content_type.starts_with("text/markdown") || content_type.starts_with("text/plain")) {
        return Err(GuideError::InvalidText);
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_GUIDE_CONTENT_BYTES as u64)
    {
        return Err(GuideError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_GUIDE_CONTENT_BYTES as u64) as usize,
    );
    response
        .by_ref()
        .take((MAX_GUIDE_CONTENT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| GuideError::Network)?;
    if bytes.len() > MAX_GUIDE_CONTENT_BYTES {
        return Err(GuideError::TooLarge);
    }
    let body = String::from_utf8(bytes).map_err(|_| GuideError::InvalidText)?;
    validate_markdown(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_online_urls_require_https_known_host_and_display_path() {
        assert!(validate_online_guide_url(
            "https://buttonscli.com/dsp/guide.md"
        ));
        assert!(validate_online_guide_url(
            "https://www.buttonscli.com/dsp/help/current.md"
        ));
        for url in [
            "http://buttonscli.com/dsp/guide.md",
            "https://buttonscli.com.evil.example/dsp/guide.md",
            "https://buttonscli.com/other/guide.md",
            "https://user@buttonscli.com/dsp/guide.md",
            "https://buttonscli.com:444/dsp/guide.md",
            "https://buttonscli.com/dsp/guide.md?next=https://evil.example",
            "javascript:alert(1)",
        ] {
            assert!(!validate_online_guide_url(url), "{url}");
        }
        let long_url = format!("https://buttonscli.com/dsp/{}", "a".repeat(2_100));
        assert!(!validate_online_guide_url(&long_url));
    }

    #[test]
    fn external_browser_links_are_https_and_host_allowlisted() {
        assert!(validate_external_url(
            "https://buttonscli.com/dsp/vote.html"
        ));
        assert!(!validate_external_url(
            "http://buttonscli.com/dsp/vote.html"
        ));
        assert!(!validate_external_url("https://wuu73.org/vibe"));
        assert!(!validate_external_url(
            "https://buttonscli.com@evil.example/"
        ));
    }

    #[test]
    fn markdown_content_is_bounded_utf8_text_and_normalizes_line_endings() {
        assert_eq!(
            validate_markdown("# Guide\r\n\r\nHello\rworld".into()).unwrap(),
            "# Guide\n\nHello\nworld"
        );
        assert_eq!(
            validate_markdown("x".repeat(MAX_GUIDE_CONTENT_BYTES + 1)),
            Err(GuideError::TooLarge)
        );
        assert_eq!(
            validate_markdown("\0bad".into()),
            Err(GuideError::InvalidText)
        );
        assert_eq!(
            validate_markdown("bad\u{7}".into()),
            Err(GuideError::InvalidText)
        );
        assert_eq!(
            validate_markdown("  \n".into()),
            Err(GuideError::InvalidText)
        );
    }

    #[test]
    fn markdown_renderer_keeps_html_and_links_as_inert_text() {
        assert_eq!(
            markdown_blocks(
                "# Guide\n- Item\n<script>alert(1)</script>\n[run](javascript:alert(1))"
            ),
            vec![
                MarkdownBlock::Heading {
                    level: 1,
                    text: "Guide".into()
                },
                MarkdownBlock::Bullet("Item".into()),
                MarkdownBlock::Paragraph("<script>alert(1)</script>".into()),
                MarkdownBlock::Paragraph("[run](javascript:alert(1))".into())
            ]
        );
    }

    #[test]
    fn markdown_block_count_is_bounded_for_pathological_newline_content() {
        let body = "\n".repeat(MAX_GUIDE_LINES + 100);
        let blocks = markdown_blocks(&body);
        assert_eq!(blocks.len(), MAX_GUIDE_LINES + 1);
        assert_eq!(blocks.last(), Some(&MarkdownBlock::Truncated));
    }

    #[test]
    fn online_fetch_client_does_not_follow_redirects() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration as StdDuration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut first, _) = listener.accept().unwrap();
            let mut request = [0_u8; 1_024];
            let _ = first.read(&mut request).unwrap();
            write!(
                first,
                "HTTP/1.1 302 Found\r\nLocation: http://{address}/redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            drop(first);

            listener.set_nonblocking(true).unwrap();
            thread::sleep(StdDuration::from_millis(150));
            match listener.accept() {
                Ok((mut redirected, _)) => {
                    let _ = redirected.read(&mut request).unwrap();
                    redirected
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 4\r\nConnection: close\r\n\r\nfake",
                        )
                        .unwrap();
                    true
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => false,
                Err(error) => panic!("redirect probe failed: {error}"),
            }
        });
        let client = guide_http_client(true).unwrap();
        assert_eq!(
            fetch_markdown_with_client(&client, &format!("http://{address}/guide.md")),
            Err(GuideError::HttpStatus)
        );
        assert!(!server.join().unwrap());
    }
}

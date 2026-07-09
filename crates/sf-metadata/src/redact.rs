//! Minimal, dependency-free secret redaction for error/log text.
//!
//! `sf-metadata`'s SOAP envelopes embed the session id (bearer token)
//! verbatim in the request body (`<sessionId>{token}</sessionId>`) — if a
//! fault/error response ever echoes request content back (some SOAP APIs do,
//! for debugging), or a caller logs a raw response body, that token could
//! leak into error text. This is applied at error-construction time in this
//! crate as defense in depth, independent of whatever the caller does with
//! the resulting `Error`.
//!
//! Deliberately narrow in scope (just the shape this crate's own SOAP
//! envelopes can leak — a Salesforce session id) rather than a general
//! secret-detection library; a fuller rollout across sf-client/sf-auth/
//! sf-rest/sf-bulk/sf-tooling is a reasonable follow-up, not done here.

const REDACTED: &str = "[REDACTED]";

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'
}

/// Redact Salesforce session/access tokens: `00D` + 12-15 alnum (org id
/// prefix) + `!` + 20+ token chars. e.g. `00Dxx0000001abcEAA!AQEAQNPS...`.
pub fn redact_session_ids(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with("00D") {
            let prefix_start = i + 3;
            let prefix_len = s[prefix_start..]
                .char_indices()
                .take_while(|(_, c)| c.is_ascii_alphanumeric())
                .last()
                .map(|(idx, c)| idx + c.len_utf8())
                .unwrap_or(0);
            let bang_pos = prefix_start + prefix_len;
            if (12..=15).contains(&prefix_len) && s[bang_pos..].starts_with('!') {
                let value_start = bang_pos + 1;
                let value_len = s[value_start..]
                    .char_indices()
                    .take_while(|(_, c)| is_token_char(*c))
                    .last()
                    .map(|(idx, c)| idx + c.len_utf8())
                    .unwrap_or(0);
                if value_len >= 20 {
                    out.push_str(REDACTED);
                    i = value_start + value_len;
                    continue;
                }
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_session_id() {
        let text = "SOAP fault echoed request: <sessionId>00Dxx0000001abcEAA!AQEAQNPSomeLongTokenBody123</sessionId>";
        let out = redact_session_ids(text);
        assert!(!out.contains("AQEAQNPSomeLongTokenBody123"));
        assert!(out.contains(REDACTED));
    }

    #[test]
    fn leaves_bare_record_ids_untouched() {
        let text = "ScratchOrgInfo 00DQL00000XUFu0AAF not found";
        assert_eq!(redact_session_ids(text), text);
    }
}

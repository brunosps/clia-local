#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretMarkerOccurrence {
    pub marker: &'static str,
    pub index: usize,
}

const ASSIGNMENT_MARKERS: [&str; 4] = ["api_key=", "apikey=", "secret=", "password="];
const BEARER_MARKER: &str = "bearer ";

pub fn first_blocking_secret_marker(text: &str) -> Option<SecretMarkerOccurrence> {
    let lower = text.to_ascii_lowercase();
    next_secret_marker(text, &lower, 0)
}

fn next_secret_marker(text: &str, lower: &str, offset: usize) -> Option<SecretMarkerOccurrence> {
    ASSIGNMENT_MARKERS
        .iter()
        .copied()
        .chain(std::iter::once(BEARER_MARKER))
        .filter_map(|marker| next_blocking_marker_occurrence(text, lower, offset, marker))
        .min_by_key(|hit| hit.index)
}

fn next_blocking_marker_occurrence(
    text: &str,
    lower: &str,
    offset: usize,
    marker: &'static str,
) -> Option<SecretMarkerOccurrence> {
    let mut search_offset = offset.min(lower.len());
    while let Some(relative_index) = lower[search_offset..].find(marker) {
        let index = search_offset + relative_index;
        let value_start = index + marker.len();
        let blocks = if marker == BEARER_MARKER {
            bearer_value_is_blocking(&text[value_start..])
        } else {
            !secret_assignment_is_placeholder(&text[value_start..])
        };
        if blocks {
            return Some(SecretMarkerOccurrence { marker, index });
        }
        search_offset = value_start;
    }
    None
}

pub fn contains_blocking_secret_marker(text: &str) -> bool {
    first_blocking_secret_marker(text).is_some()
}

fn secret_assignment_is_placeholder(tail: &str) -> bool {
    let line = tail
        .split_once(['\n', '\r'])
        .map(|(line, _)| line)
        .unwrap_or(tail)
        .trim_start();
    if line.is_empty() || line.starts_with('#') {
        return true;
    }
    let token = line
        .split_once(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ';' | '&' | '|'))
        .map(|(token, _)| token)
        .unwrap_or(line)
        .trim()
        .trim_matches(',');
    if token.is_empty() {
        return true;
    }
    let token = strip_matching_quotes(token);
    let lower = token.to_ascii_lowercase();
    lower == "xxx"
        || lower == "changeme"
        || token.starts_with('{')
        || (token.starts_with("${") && token.contains('}'))
        || shell_env_reference_is_placeholder(token)
        || (token.starts_with('<') && token.contains('>'))
}

fn strip_matching_quotes(token: &str) -> &str {
    [('"', '"'), ('`', '`')]
        .into_iter()
        .find_map(|(open, close)| {
            token
                .strip_prefix(open)
                .and_then(|inner| inner.strip_suffix(close))
        })
        .unwrap_or(token)
}

fn shell_env_reference_is_placeholder(token: &str) -> bool {
    let Some(name) = token.strip_prefix('$') else {
        return false;
    };
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_uppercase())
        && chars.all(|ch| ch == '_' || ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn bearer_value_is_blocking(tail: &str) -> bool {
    let line = tail
        .split_once(['\n', '\r'])
        .map(|(line, _)| line)
        .unwrap_or(tail);
    let value = line.trim_start();
    let Some(first) = value.chars().next() else {
        return false;
    };
    if matches!(first, '{' | '"' | '\'' | '$') || value.starts_with("%s") {
        return false;
    }
    let token = value
        .split_once(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ';' | '&' | '|'))
        .map(|(token, _)| token)
        .unwrap_or(value)
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | ',' | ')' | '}' | ']'));
    token.len() >= 8 && token.chars().all(is_token_literal_char)
}

fn is_token_literal_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | '+' | '=')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_assignments_block_only_literal_values() {
        for content in [
            "PASSWORD=",
            "password=   \n",
            "secret=${APP_SECRET}",
            "api_key=$API_KEY",
            "password=\"$PASSWORD\"",
            "PASSWORD={password}",
            "apikey=<placeholder>",
            "password=xxx",
            "secret=ChangeMe",
        ] {
            assert!(!contains_blocking_secret_marker(content), "{content}");
        }
        for content in [
            "password=hunter2",
            "password='$ecret123'",
            "secret=real-value",
            "api_key=sk-live",
            "apikey=abc12345",
        ] {
            assert!(contains_blocking_secret_marker(content), "{content}");
        }
    }

    #[test]
    fn bearer_blocks_literal_tokens_only() {
        for content in [
            "format!(\"Bearer {token}\")",
            "format!(\"Bearer {}\", token)",
            "Authorization: Bearer $TOKEN",
            "Authorization: Bearer ",
            "Authorization: Bearer \nnotasecretword",
            "Authorization: Bearer %s",
            "Authorization: Bearer token",
        ] {
            assert!(!contains_blocking_secret_marker(content), "{content}");
        }
        assert!(contains_blocking_secret_marker(
            "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9"
        ));
    }

    #[test]
    fn scans_past_placeholder_occurrences_for_later_assignment_secrets() {
        let content = "SMTP_PASSWORD=\nSMTP_SECRET=${SMTP_SECRET}\nADMIN_PASSWORD=hunter2\n";
        let hit = first_blocking_secret_marker(content).expect("later real secret blocks");
        assert_eq!(hit.marker, "password=");
        assert!(content[hit.index..].starts_with("PASSWORD=hunter2"));
    }

    #[test]
    fn scans_past_placeholder_occurrences_for_later_bearer_tokens() {
        let content =
            "let header = format!(\"Bearer {token}\");\nAuthorization: Bearer eyJhbGciOiJIUzI1NiJ9\n";
        let hit = first_blocking_secret_marker(content).expect("later real bearer blocks");
        assert_eq!(hit.marker, "bearer ");
        assert!(content[hit.index..].starts_with("Bearer eyJhbGciOiJIUzI1NiJ9"));
    }
}

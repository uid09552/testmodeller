//! Implementation and backlog links of test cases (FR-025, FR-026).
//!
//! A link is a full http(s) URL. A backlog item is identified by its
//! *normalised* URL, so spellings that differ only in scheme or host case, a
//! fragment or a trailing slash group as one item.

/// Maximum length of a stored link.
pub const MAX_LINK_LEN: usize = 2048;

/// Why a link was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    /// Longer than [`MAX_LINK_LEN`] characters.
    #[error("must be at most {MAX_LINK_LEN} characters")]
    TooLong,
    /// Not an `http://` or `https://` URL with a host.
    #[error("must be a full http(s) URL")]
    NotHttpUrl,
}

/// Splits `url` into lower-cased scheme, authority and the rest, if it is http(s) with a host.
fn split(url: &str) -> Option<(String, &str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let host = authority.rsplit('@').next().unwrap_or("");
    let host_name = host.split(':').next().unwrap_or("");
    if host_name.is_empty() || host_name.starts_with('[') && !host.contains(']') {
        return None;
    }
    Some((scheme, authority, tail))
}

/// Validates a link and returns it trimmed. The raw spelling is kept.
pub fn validate_link(raw: &str) -> Result<String, LinkError> {
    let url = raw.trim();
    if url.chars().count() > MAX_LINK_LEN {
        return Err(LinkError::TooLong);
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) || split(url).is_none() {
        return Err(LinkError::NotHttpUrl);
    }
    Ok(url.to_owned())
}

/// Grouping key of a backlog item: lower-case scheme and host, no fragment, no trailing slash,
/// query kept. `None` when `url` is not a valid link.
pub fn backlog_key(url: &str) -> Option<String> {
    let url = validate_link(url).ok()?;
    let (scheme, authority, tail) = split(&url)?;
    let tail = tail.split('#').next().unwrap_or("");
    let (path, query) = match tail.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (tail, None),
    };
    let authority = match authority.rsplit_once('@') {
        Some((user, host)) => format!("{user}@{}", host.to_ascii_lowercase()),
        None => authority.to_ascii_lowercase(),
    };
    let path = path.trim_end_matches('/');
    let mut key = format!("{scheme}://{authority}{path}");
    if let Some(q) = query.filter(|q| !q.is_empty()) {
        key.push('?');
        key.push_str(q);
    }
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_http_and_https() {
        assert_eq!(
            validate_link(" https://ex.com/a/1 ").as_deref(),
            Ok("https://ex.com/a/1")
        );
        assert!(validate_link("http://localhost:8080/x").is_ok());
    }

    #[test]
    fn rejects_other_schemes_and_junk() {
        for bad in [
            "",
            "ex.com/a",
            "ftp://ex.com",
            "javascript:alert(1)",
            "https://",
            "https:///path",
            "https://ex .com",
            "mailto:a@b.c",
        ] {
            assert_eq!(validate_link(bad), Err(LinkError::NotHttpUrl), "{bad}");
        }
    }

    #[test]
    fn rejects_too_long() {
        let long = format!("https://ex.com/{}", "a".repeat(MAX_LINK_LEN));
        assert_eq!(validate_link(&long), Err(LinkError::TooLong));
    }

    #[test]
    fn equal_after_normalisation() {
        let key = backlog_key("https://ex.com/browse/TM-1").unwrap();
        for same in [
            "HTTPS://EX.com/browse/TM-1",
            "https://ex.com/browse/TM-1/",
            "https://ex.com/browse/TM-1#comment-2",
            "https://ex.com/browse/TM-1/#x",
        ] {
            assert_eq!(backlog_key(same).as_deref(), Some(key.as_str()), "{same}");
        }
    }

    #[test]
    fn path_case_and_query_are_kept() {
        assert_ne!(
            backlog_key("https://ex.com/TM-1"),
            backlog_key("https://ex.com/tm-1")
        );
        assert_ne!(
            backlog_key("https://ex.com/i?id=1"),
            backlog_key("https://ex.com/i?id=2")
        );
        assert_eq!(
            backlog_key("https://ex.com/i/?id=1#f").as_deref(),
            Some("https://ex.com/i?id=1")
        );
    }

    #[test]
    fn invalid_has_no_key() {
        assert_eq!(backlog_key("nope"), None);
    }
}

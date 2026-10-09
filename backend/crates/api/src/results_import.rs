//! Parsing of JUnit XML and Cucumber JSON result files, and matching results to test
//! cases (FR-050, FR-051). Pure: no I/O. See docs/specification/06-ui.md#test-results.
//!
//! Safety: files are limited to [`MAX_FILE_BYTES`] and [`MAX_RESULTS`] results; XML with a
//! DOCTYPE is rejected outright, so no DTD, external entity or entity expansion is ever
//! processed (quick-xml only knows the five predefined entities); nesting is capped.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde_json::Value;
use tm_domain::ResultStatus;
use uuid::Uuid;

/// Largest accepted file.
pub const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
/// Most results accepted from one file.
pub const MAX_RESULTS: usize = 100_000;
/// Deepest accepted XML nesting.
const MAX_DEPTH: usize = 64;
/// Longest stored message.
const MAX_MESSAGE_CHARS: usize = 4000;

/// Supported result formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultFormat {
    /// JUnit XML (`<testsuites>` / `<testsuite>`).
    Junit,
    /// Cucumber JSON (array of features).
    Cucumber,
}

impl ResultFormat {
    /// Wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Junit => "junit",
            Self::Cucumber => "cucumber",
        }
    }
}

/// Why a file was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Over [`MAX_FILE_BYTES`].
    #[error("the file is larger than {} MB", MAX_FILE_BYTES / 1024 / 1024)]
    TooLarge,
    /// Over [`MAX_RESULTS`].
    #[error("the file has more than {MAX_RESULTS} results")]
    TooMany,
    /// XML declaring a document type (DTD).
    #[error("XML with a DOCTYPE is not accepted")]
    Doctype,
    /// Not well-formed, or not the declared format.
    #[error("{0}")]
    Malformed(String),
}

/// One result read from a file, before matching.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedResult {
    /// Test name as the runner reported it.
    pub name: String,
    /// Test case id from an `@tm-<id>` tag or `tmId` property, if any.
    pub tm_id: Option<Uuid>,
    /// Outcome.
    pub status: ResultStatus,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Failure or error message.
    pub message: Option<String>,
    /// When it ran, if the file says.
    pub executed_at: Option<DateTime<Utc>>,
}

/// A parsed file.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFile {
    /// Run identity: from the file's own timestamp, else a hash of its content.
    pub run_id: String,
    /// Results in file order.
    pub results: Vec<ParsedResult>,
}

/// Parses `bytes` as `format`.
pub fn parse(format: ResultFormat, bytes: &[u8]) -> Result<ParsedFile, ParseError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(ParseError::TooLarge);
    }
    let (results, run) = match format {
        ResultFormat::Junit => parse_junit(bytes)?,
        ResultFormat::Cucumber => parse_cucumber(bytes)?,
    };
    let run_id = match run {
        Some(r) => format!("{}:{r}", format.as_str()),
        None => format!("{}:{:016x}", format.as_str(), fnv1a(bytes)),
    };
    Ok(ParsedFile { run_id, results })
}

/// FNV-1a: a stable content hash (unlike `DefaultHasher`, it never changes between builds).
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The id in the first `@tm-<uuid>` of `text`.
pub fn find_tag(text: &str) -> Option<Uuid> {
    text.match_indices("@tm-").find_map(|(i, _)| {
        text.get(i + 4..i + 40)
            .and_then(|s| Uuid::parse_str(s).ok())
    })
}

fn truncate(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.chars().take(MAX_MESSAGE_CHARS).collect())
}

fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|n| n.and_utc())
        })
}

fn secs_to_ms(s: &str) -> Option<i64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
        .map(|v| (v * 1000.0).round() as i64)
}

// ── JUnit ─────────────────────────────────────────────────────────────────────

fn attr(e: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, ParseError> {
    for a in e.attributes() {
        let a = a.map_err(|e| ParseError::Malformed(format!("invalid XML attribute: {e}")))?;
        if a.key.as_ref() == name {
            let v = a
                .unescape_value()
                .map_err(|e| ParseError::Malformed(format!("invalid XML attribute: {e}")))?;
            return Ok(Some(v.into_owned()));
        }
    }
    Ok(None)
}

struct OpenCase {
    result: ParsedResult,
    classname: String,
    /// Collect text of the failure/error element when it has no `message`.
    in_problem: bool,
}

fn parse_junit(bytes: &[u8]) -> Result<(Vec<ParsedResult>, Option<String>), ParseError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut results = Vec::new();
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut suite_time: Option<DateTime<Utc>> = None;
    let mut run: Option<String> = None;
    let mut case: Option<OpenCase> = None;

    loop {
        let event = reader.read_event_into(&mut buf).map_err(|e| {
            ParseError::Malformed(format!(
                "not well-formed XML at byte {}: {e}",
                reader.buffer_position()
            ))
        })?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::DocType(_) => return Err(ParseError::Doctype),
            Event::Start(ref e) | Event::Empty(ref e) => {
                if !empty {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err(ParseError::Malformed("XML is nested too deeply".into()));
                    }
                }
                let tag = e.name().as_ref().to_vec();
                if !root_seen {
                    root_seen = true;
                    if tag != b"testsuites" && tag != b"testsuite" {
                        return Err(ParseError::Malformed(
                            "not a JUnit XML file: the root must be <testsuites> or <testsuite>"
                                .into(),
                        ));
                    }
                }
                match tag.as_slice() {
                    b"testsuite" => {
                        let ts = attr(e, b"timestamp")?;
                        suite_time = ts.as_deref().and_then(parse_time);
                        if run.is_none() {
                            if let Some(ts) = ts {
                                let name = attr(e, b"name")?.unwrap_or_default();
                                run = Some(format!("{name}@{ts}"));
                            }
                        }
                    }
                    b"testcase" => {
                        let name = attr(e, b"name")?.unwrap_or_default();
                        let classname = attr(e, b"classname")?.unwrap_or_default();
                        let open = OpenCase {
                            result: ParsedResult {
                                tm_id: find_tag(&name).or_else(|| find_tag(&classname)),
                                name,
                                status: ResultStatus::Passed,
                                duration_ms: attr(e, b"time")?.as_deref().and_then(secs_to_ms),
                                message: None,
                                executed_at: suite_time,
                            },
                            classname,
                            in_problem: false,
                        };
                        if empty {
                            push(&mut results, open.result)?;
                        } else {
                            case = Some(open);
                        }
                    }
                    b"failure" | b"error" | b"skipped" => {
                        if let Some(c) = case.as_mut() {
                            c.result.status = match tag.as_slice() {
                                b"failure" => ResultStatus::Failed,
                                b"error" => ResultStatus::Error,
                                _ => ResultStatus::Skipped,
                            };
                            c.result.message = attr(e, b"message")?.as_deref().and_then(truncate);
                            c.in_problem = !empty && c.result.message.is_none();
                        }
                    }
                    b"property" => {
                        if let Some(c) = case.as_mut() {
                            let name = attr(e, b"name")?.unwrap_or_default();
                            if matches!(name.as_str(), "tmId" | "tm-id" | "tm_id") {
                                let value = attr(e, b"value")?.unwrap_or_default();
                                let value = value.trim().trim_start_matches("@tm-");
                                if let Ok(id) = Uuid::parse_str(value) {
                                    c.result.tm_id = Some(id);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) => {
                if let Some(c) = case.as_mut().filter(|c| c.in_problem) {
                    let text = t
                        .unescape()
                        .map_err(|e| ParseError::Malformed(format!("invalid XML text: {e}")))?;
                    c.result.message = truncate(&text);
                }
            }
            Event::CData(t) => {
                if let Some(c) = case.as_mut().filter(|c| c.in_problem) {
                    c.result.message = truncate(&String::from_utf8_lossy(&t));
                }
            }
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                match e.name().as_ref() {
                    b"failure" | b"error" | b"skipped" => {
                        if let Some(c) = case.as_mut() {
                            c.in_problem = false;
                        }
                    }
                    b"testcase" => {
                        if let Some(c) = case.take() {
                            let mut r = c.result;
                            if r.tm_id.is_none() {
                                r.tm_id = find_tag(&c.classname);
                            }
                            push(&mut results, r)?;
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if depth != 0 {
        return Err(ParseError::Malformed(
            "the XML ends before all its elements are closed (truncated file?)".into(),
        ));
    }
    if !root_seen {
        return Err(ParseError::Malformed("the file is empty".into()));
    }
    Ok((results, run))
}

fn push(results: &mut Vec<ParsedResult>, r: ParsedResult) -> Result<(), ParseError> {
    if results.len() >= MAX_RESULTS {
        return Err(ParseError::TooMany);
    }
    results.push(r);
    Ok(())
}

// ── Cucumber ──────────────────────────────────────────────────────────────────

fn parse_cucumber(bytes: &[u8]) -> Result<(Vec<ParsedResult>, Option<String>), ParseError> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|e| ParseError::Malformed(format!("not valid JSON: {e}")))?;
    let features = root.as_array().ok_or_else(|| {
        ParseError::Malformed("not a Cucumber JSON file: expected an array of features".into())
    })?;
    let mut results = Vec::new();
    let mut run = None;
    for feature in features {
        let Some(elements) = feature.get("elements").and_then(Value::as_array) else {
            continue;
        };
        for el in elements {
            if el.get("type").and_then(Value::as_str) == Some("background") {
                continue;
            }
            let name = el
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let tm_id = el
                .get("tags")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|t| t.get("name").and_then(Value::as_str))
                .find_map(find_tag);
            let started = el.get("start_timestamp").and_then(Value::as_str);
            if run.is_none() {
                run = started.map(str::to_owned);
            }

            let mut status = ResultStatus::Passed;
            let mut message = None;
            let mut nanos: i64 = 0;
            let mut timed = false;
            let hooks_and_steps = ["before", "steps", "after"]
                .into_iter()
                .filter_map(|k| el.get(k).and_then(Value::as_array))
                .flatten();
            for step in hooks_and_steps {
                let Some(res) = step.get("result") else {
                    continue;
                };
                if let Some(d) = res.get("duration").and_then(Value::as_i64) {
                    nanos = nanos.saturating_add(d.max(0));
                    timed = true;
                }
                let step_status = match res.get("status").and_then(Value::as_str) {
                    Some("passed") => ResultStatus::Passed,
                    Some("failed") => ResultStatus::Failed,
                    Some("ambiguous") => ResultStatus::Error,
                    _ => ResultStatus::Skipped,
                };
                if rank(step_status) > rank(status) {
                    status = step_status;
                    message = res
                        .get("error_message")
                        .and_then(Value::as_str)
                        .and_then(truncate);
                }
            }
            push(
                &mut results,
                ParsedResult {
                    name,
                    tm_id,
                    status,
                    duration_ms: timed.then_some(nanos / 1_000_000),
                    message,
                    executed_at: started.and_then(parse_time),
                },
            )?;
        }
    }
    Ok((results, run))
}

/// Which step outcome decides a scenario: any failure fails it.
fn rank(s: ResultStatus) -> u8 {
    match s {
        ResultStatus::Passed => 0,
        ResultStatus::Skipped => 1,
        ResultStatus::Error => 2,
        ResultStatus::Failed => 3,
    }
}

// ── Matching ──────────────────────────────────────────────────────────────────

/// How a result was matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// To this test case.
    Found(Uuid),
    /// Its tag names no test case of the project, or its name matches none.
    Unmatched,
    /// Its name matches several test cases.
    Ambiguous,
}

/// Test cases of one project, for matching.
pub struct Candidates {
    ids: std::collections::HashSet<Uuid>,
    by_name: HashMap<String, Vec<Uuid>>,
}

impl Candidates {
    /// From `(id, name)` of every test case in the project.
    pub fn new(cases: impl IntoIterator<Item = (Uuid, String)>) -> Self {
        let mut ids = std::collections::HashSet::new();
        let mut by_name: HashMap<String, Vec<Uuid>> = HashMap::new();
        for (id, name) in cases {
            ids.insert(id);
            by_name.entry(name).or_default().push(id);
        }
        Self { ids, by_name }
    }

    /// The tag decides when present; otherwise an exact, unique name.
    pub fn resolve(&self, r: &ParsedResult) -> Match {
        if let Some(id) = r.tm_id {
            return if self.ids.contains(&id) {
                Match::Found(id)
            } else {
                Match::Unmatched
            };
        }
        match self.by_name.get(&r.name).map(Vec::as_slice) {
            Some([id]) => Match::Found(*id),
            Some([_, _, ..]) => Match::Ambiguous,
            _ => Match::Unmatched,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "5f0c7c1e-8a6f-4b5e-9a35-1d2c3b4a5f60";

    const JUNIT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="jest tests" tests="4" failures="1" errors="1" time="1.234">
  <testsuite name="login" timestamp="2026-10-01T09:30:00" tests="4" failures="1" errors="1" skipped="1" time="1.234">
    <testcase classname="login" name="Valid login @tm-5f0c7c1e-8a6f-4b5e-9a35-1d2c3b4a5f60" time="0.120"/>
    <testcase classname="login" name="Wrong password" time="0.200">
      <failure message="expected &quot;error&quot; to be shown" type="AssertionError">stack trace…</failure>
    </testcase>
    <testcase classname="login" name="Lockout" time="0.010">
      <error><![CDATA[TypeError: boom]]></error>
    </testcase>
    <testcase classname="login" name="Reset" time="0">
      <skipped/>
      <properties><property name="tmId" value="@tm-5f0c7c1e-8a6f-4b5e-9a35-1d2c3b4a5f60"/></properties>
    </testcase>
  </testsuite>
</testsuites>"#;

    #[test]
    fn junit_reads_status_duration_message_tag_and_time() {
        let f = parse(ResultFormat::Junit, JUNIT.as_bytes()).unwrap();
        assert_eq!(f.run_id, "junit:login@2026-10-01T09:30:00");
        let r = &f.results;
        assert_eq!(r.len(), 4);
        assert_eq!(r[0].status, ResultStatus::Passed);
        assert_eq!(r[0].tm_id, Some(Uuid::parse_str(ID).unwrap()));
        assert_eq!(r[0].duration_ms, Some(120));
        assert_eq!(
            r[0].executed_at.unwrap().to_rfc3339(),
            "2026-10-01T09:30:00+00:00"
        );
        assert_eq!(r[1].status, ResultStatus::Failed);
        assert_eq!(
            r[1].message.as_deref(),
            Some("expected \"error\" to be shown")
        );
        assert_eq!(r[1].tm_id, None);
        assert_eq!(r[2].status, ResultStatus::Error);
        assert_eq!(r[2].message.as_deref(), Some("TypeError: boom"));
        assert_eq!(r[3].status, ResultStatus::Skipped);
        assert_eq!(r[3].tm_id, Some(Uuid::parse_str(ID).unwrap()));
    }

    #[test]
    fn junit_without_timestamp_uses_a_stable_content_hash() {
        let xml = br#"<testsuite name="s"><testcase name="a"/></testsuite>"#;
        let a = parse(ResultFormat::Junit, xml).unwrap();
        let b = parse(ResultFormat::Junit, xml).unwrap();
        assert_eq!(a.run_id, b.run_id);
        assert!(a.run_id.starts_with("junit:"));
        assert_eq!(a.results[0].executed_at, None);
    }

    #[test]
    fn rejects_entity_expansion_and_external_entities() {
        let billion_laughs = br#"<?xml version="1.0"?>
<!DOCTYPE lolz [<!ENTITY lol "lol"><!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">]>
<testsuite name="x"><testcase name="&lol2;"/></testsuite>"#;
        assert_eq!(
            parse(ResultFormat::Junit, billion_laughs),
            Err(ParseError::Doctype)
        );
        let xxe = br#"<?xml version="1.0"?><!DOCTYPE t [<!ENTITY x SYSTEM "file:///etc/passwd">]>
<testsuite name="x"><testcase name="&x;"/></testsuite>"#;
        assert_eq!(parse(ResultFormat::Junit, xxe), Err(ParseError::Doctype));
        // An undeclared entity is not expanded either.
        let undeclared = br#"<testsuite name="x"><testcase name="&x;"/></testsuite>"#;
        assert!(matches!(
            parse(ResultFormat::Junit, undeclared),
            Err(ParseError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_malformed_and_foreign_xml() {
        for bad in [
            &b"<testsuite><testcase name='a'>"[..],
            b"<html><body/></html>",
            b"",
            b"not xml at all",
        ] {
            assert!(
                matches!(
                    parse(ResultFormat::Junit, bad),
                    Err(ParseError::Malformed(_))
                ),
                "{}",
                String::from_utf8_lossy(bad)
            );
        }
        let deep = format!(
            "<testsuite>{}{}</testsuite>",
            "<a>".repeat(100),
            "</a>".repeat(100)
        );
        assert!(matches!(
            parse(ResultFormat::Junit, deep.as_bytes()),
            Err(ParseError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_oversized_files() {
        let big = vec![b' '; MAX_FILE_BYTES + 1];
        assert_eq!(parse(ResultFormat::Junit, &big), Err(ParseError::TooLarge));
        assert_eq!(
            parse(ResultFormat::Cucumber, &big),
            Err(ParseError::TooLarge)
        );
    }

    const CUCUMBER: &str = r#"[
  {
    "uri": "features/login.feature", "name": "Login",
    "elements": [
      { "type": "background", "name": "", "steps": [ { "result": { "status": "passed", "duration": 1000000 } } ] },
      { "type": "scenario", "name": "Valid login", "start_timestamp": "2026-10-02T08:00:00.000Z",
        "tags": [ { "name": "@smoke" }, { "name": "@tm-5f0c7c1e-8a6f-4b5e-9a35-1d2c3b4a5f60" } ],
        "steps": [
          { "result": { "status": "passed", "duration": 2000000 } },
          { "result": { "status": "passed", "duration": 3500000 } } ] },
      { "type": "scenario", "name": "Wrong password",
        "steps": [
          { "result": { "status": "passed", "duration": 1000000 } },
          { "result": { "status": "failed", "duration": 1000000, "error_message": "expected error" } },
          { "result": { "status": "skipped" } } ] },
      { "type": "scenario", "name": "Pending one",
        "steps": [ { "result": { "status": "undefined" } } ] },
      { "type": "scenario", "name": "Hook fails",
        "before": [ { "result": { "status": "failed", "error_message": "db down" } } ],
        "steps": [ { "result": { "status": "skipped" } } ] }
    ]
  }
]"#;

    #[test]
    fn cucumber_fails_a_scenario_when_any_step_fails() {
        let f = parse(ResultFormat::Cucumber, CUCUMBER.as_bytes()).unwrap();
        assert_eq!(f.run_id, "cucumber:2026-10-02T08:00:00.000Z");
        let r = &f.results;
        assert_eq!(r.len(), 4, "backgrounds are not results");
        assert_eq!(r[0].status, ResultStatus::Passed);
        assert_eq!(r[0].tm_id, Some(Uuid::parse_str(ID).unwrap()));
        assert_eq!(r[0].duration_ms, Some(5));
        assert!(r[0].executed_at.is_some());
        assert_eq!(r[1].status, ResultStatus::Failed);
        assert_eq!(r[1].message.as_deref(), Some("expected error"));
        assert_eq!(r[2].status, ResultStatus::Skipped);
        assert_eq!(r[3].status, ResultStatus::Failed);
        assert_eq!(r[3].message.as_deref(), Some("db down"));
    }

    #[test]
    fn rejects_malformed_cucumber() {
        for bad in [&b"{\"not\": \"an array\"}"[..], b"[{", b""] {
            assert!(matches!(
                parse(ResultFormat::Cucumber, bad),
                Err(ParseError::Malformed(_))
            ));
        }
    }

    fn result(name: &str, tm_id: Option<Uuid>) -> ParsedResult {
        ParsedResult {
            name: name.into(),
            tm_id,
            status: ResultStatus::Passed,
            duration_ms: None,
            message: None,
            executed_at: None,
        }
    }

    #[test]
    fn matches_by_tag_then_unique_name() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let cands = Candidates::new([
            (a, "Valid login".to_owned()),
            (b, "Twice".to_owned()),
            (c, "Twice".to_owned()),
        ]);
        // The tag wins over the name.
        assert_eq!(cands.resolve(&result("Twice", Some(a))), Match::Found(a));
        assert_eq!(cands.resolve(&result("Valid login", None)), Match::Found(a));
        assert_eq!(cands.resolve(&result("Twice", None)), Match::Ambiguous);
        assert_eq!(cands.resolve(&result("Nobody", None)), Match::Unmatched);
        // A tag of a test case outside the project matches nothing, even by name.
        assert_eq!(
            cands.resolve(&result("Valid login", Some(Uuid::new_v4()))),
            Match::Unmatched
        );
    }

    #[test]
    fn finds_tags_anywhere_in_text() {
        assert_eq!(
            find_tag(&format!("Login works @tm-{ID} [chromium]")),
            Some(Uuid::parse_str(ID).unwrap())
        );
        assert_eq!(find_tag("@tm-not-a-uuid"), None);
        assert_eq!(find_tag("plain"), None);
    }
}

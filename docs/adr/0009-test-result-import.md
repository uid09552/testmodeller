---
title: 0009. Importing test results: matching by tag, parsing safely
type: adr
status: accepted
date: 2026-10-09
tags: [backend, api, security, test-results]
related: [../specification/06-ui.md, ../specification/02-domain-model.md, ../../openspec/changes/test-results-import/design.md]
---

# 0009. Importing test results: matching by tag, parsing safely

## Context

Teams run their tests outside TestModeller and want coverage to say "passing",
not only "modelled". Result files (JUnit XML, Cucumber JSON) come from
untrusted runners and name tests in their own way; test case names are neither
unique nor stable.

## Decision

- A result is matched by the tag `@tm-<test case id>`, which the Gherkin export
  puts on every scenario and the CSV export carries in a `tmId` column;
  otherwise by exact, unique test case name within the project. A tag naming a
  test case outside the project matches nothing.
- Results are stored per test case and run (`UNIQUE (test_case_id, run_id)`), so
  re-importing a file adds nothing. The run id comes from the file (JUnit suite
  name and timestamp, Cucumber start time), else from an FNV-1a hash of its
  content, which, unlike Rust's `DefaultHasher`, is stable across builds.
- JUnit XML is parsed with `quick-xml` (new dependency). Any DOCTYPE is
  rejected, so no DTD, external entity or entity expansion is ever processed;
  quick-xml itself only knows the five predefined entities. Files are capped at
  10 MB, 100,000 results and 64 levels of nesting, and truncated XML is
  rejected.
- The upload is the raw request body, not multipart; the file name travels in
  `Content-Disposition`.

## Consequences

- Runners that drop the tag still match by name; ambiguous and unmatched
  results are listed in the report instead of being guessed.
- A new XML parser is a supply-chain addition; `quick-xml` is widely used and
  pure Rust. `cargo audit` covers it in CI.
- Rejecting every DOCTYPE could refuse an exotic but harmless report; no common
  JUnit producer emits one.

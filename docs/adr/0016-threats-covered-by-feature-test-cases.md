---
title: 0016. Feature test cases can cover threats; security tests convert into them
type: adr
status: accepted
date: 2026-11-28
tags: [threat-model, testing, traceability, api]
related: [0011-threat-model-entities-and-security-tests.md, ../specification/09-threat-modeling.md, ../specification/05-api.md]
---

# 0016. Feature test cases can cover threats; security tests convert into them

## Context

ADR 0011 kept security tests apart from feature test cases: they are derived
from threats and controls, not from a state machine, and their results are
recorded by hand. That keeps the threat model self-contained, but a threat
then has no link to the tests that actually run in CI. A team that automates a
security check as a model-based test — say, a lockout after three failed
logins, modelled as states and transitions — could not say which threat it
covers, and a failing CI run told the threat model nothing.

ADR 0011 anticipated this: generated security tests carry their template,
threats and controls "so they can be converted into feature test cases without
loss".

## Decision

- **Link.** A many-to-many link between feature test cases and threats
  (`test_case_threats`). It is edited from the test case (`threatIds` on
  `TestCaseInput`) and read back on both sides (`TestCase.threatIds`,
  `Threat.testCaseIds`). A test case may only link threats of the threat model
  of its own project. Deleting either side deletes the link. Omitting
  `threatIds` on a write keeps the current links, so existing clients are
  unaffected.
- **Results flow back.** A covering test case whose latest imported result is
  failed or error raises the threat's existing `failing-test` flag. No new flag
  and no new status: a failing automated check means the same as a failing
  manual one.
- **Convert.** `POST …/security-tests/{id}/convert` copies a security test into
  a new manual feature test case of a feature in the same project, linked to
  the same threats. The security test becomes `obsolete` and records
  `convertedToTestCaseId`, so the same check is not tracked twice. Given a
  model of the feature, the new test case is assigned to its initial state:
  the editor and the Test Cases page list only assigned test cases, so an
  unassigned copy would be invisible. Which states or transitions it really
  exercises is then a modelling decision made in the editor.
- **Security tests stay.** They remain a separate list with manual results.
  This ADR adds a path from them to feature test cases; it does not merge the
  two.

## Consequences

- Feature test cases gain one optional field; the Test Cases page, coverage,
  traceability and result import are otherwise unchanged. The Gherkin export
  tags a covering scenario with `@THR-n`.
- The threat model's computed flags now read feature test results, so a
  threat-model read joins `test_case_threats` with the latest result per test
  case. Both columns of the link table are indexed.
- Baselines keep their schema: a baseline stores flags as they were, and does
  not store the links.
- MCP `get_threat_model` returns the links with the threats, through the same
  DTOs.

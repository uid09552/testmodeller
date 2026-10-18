# Spec Delta

## Purpose

Keeps a threat model's security test cases in a list of their own, generates draft tests from threats and their controls, records manual results, and exports the tests, so every threat and control can be verified.

## ADDED Requirements

### Requirement: Security test list is separate
Security test cases SHALL belong to a threat model, not to a feature. They SHALL NOT appear on the Test Cases page, in feature or model coverage, in traceability, in feature test case exports or in test result imports, and the feature test cases SHALL NOT appear in the security test list.

#### Scenario: Not on the Test Cases page
- **WHEN** a project's threat model has five security tests
- **THEN** the Test Cases page lists none of them

#### Scenario: Deleting the threat model
- **WHEN** a project's threat model is deleted
- **THEN** its security tests are deleted and feature test cases are untouched

### Requirement: Security test records
The user SHALL be able to create, edit and delete security tests. A security test has a display id (`SEC-<n>`, sequential, never reused), title (required), objective, preconditions, ordered steps (action and expected result), linked threats (at least one), linked controls, test type (negative authorization, authentication, input validation, transport security, configuration review, resilience, logging and audit, other), status (draft, ready, obsolete) and origin (manual or generated).

#### Scenario: Create a manual test
- **WHEN** the user creates "Access another customer's record" linked to `THR-1` and `CTL-1` with two steps
- **THEN** it is saved as `SEC-1`, draft, origin manual

#### Scenario: Test without a threat
- **WHEN** a client creates a security test linked to no threat
- **THEN** the request is rejected as invalid

### Requirement: Generate security tests
"Generate tests" on the Security Tests tab SHALL create draft security tests for the selected open or accepted threats (all of them when none is selected) from versioned test templates keyed by STRIDE category and target type, filled with the threat's target, assets and linked controls. Each generated test SHALL record its template id and version and link to its threat and the controls used.

#### Scenario: Spoofing on a flow
- **WHEN** tests are generated for a spoofing threat on flow "Web app → Customer API" linked to control "Service authentication"
- **THEN** a draft test is created with steps that call "Customer API" from "Web app" without and with invalid credentials and expect rejection, linked to the threat and the control

#### Scenario: Information disclosure on a component
- **WHEN** tests are generated for an information disclosure threat on "Customer API" with asset "Customer records"
- **THEN** a draft negative-authorization test is created that requests another user's "Customer records" and expects denial

#### Scenario: One test per control
- **WHEN** a threat has three linked controls and the template is control-specific
- **THEN** one draft test is generated per control

#### Scenario: No duplicates
- **WHEN** tests are generated twice for the same threat without changes
- **THEN** the second run creates no test whose template and threat and control match an existing non-obsolete test, and reports how many were skipped

#### Scenario: Candidates excluded
- **WHEN** the user generates tests and some threats are candidates or dismissed
- **THEN** no tests are generated for them

### Requirement: Drafts are reviewed
Generated tests SHALL be created as draft and SHALL become ready only when a user marks them ready, singly or in bulk. Generation SHALL never change or delete an existing test.

#### Scenario: Bulk mark ready
- **WHEN** the user selects four drafts and chooses "Mark ready"
- **THEN** all four become ready

#### Scenario: Regeneration leaves edits alone
- **WHEN** the user edited a generated test and generates again
- **THEN** the edited test is unchanged

### Requirement: Stale generated tests
A generated test SHALL be marked stale when its threat is dismissed, when it no longer links any threat (its threats were deleted), or when a control it was generated for is unlinked from the threat. Stale tests SHALL be shown with "⚠ Stale" and the reason, and SHALL NOT be changed or deleted automatically.

#### Scenario: Threat dismissed
- **WHEN** the threat of a generated test is dismissed
- **THEN** the test is marked "⚠ Stale: threat dismissed" and remains in the list

#### Scenario: Threat deleted
- **WHEN** the only threat of a test is deleted
- **THEN** the test remains, links no threat and is marked "⚠ Stale: no threat"

### Requirement: Manual results
The user SHALL be able to record the result of a ready security test as passed, failed or not run, with an optional note and the date. The list SHALL show each test's latest result as a symbol and a word.

#### Scenario: Record a failure
- **WHEN** the user records "failed" with note "IDOR on /customers/{id}"
- **THEN** the test shows "✗ Failed", the note and today's date

#### Scenario: Failing test on the threat
- **WHEN** a threat's linked test last failed
- **THEN** the threat shows "✗ Failing test" in the Threats tab

### Requirement: Security test list and export
The Security Tests tab SHALL list tests with id, title, type, threats, controls, status and latest result, sortable and filterable by each, and SHALL export the filtered tests as CSV (one row per test, steps joined) and as Gherkin (one scenario per test, tagged `@sec-<id>` and with the threat ids).

#### Scenario: Gherkin export
- **WHEN** the user exports `SEC-1` as Gherkin
- **THEN** the file contains a scenario titled with the test's title, tagged `@sec-<id>` and `@THR-1`, with one Given per precondition, and When/Then per step

#### Scenario: Filtered export
- **WHEN** the list is filtered to failed tests and the user exports CSV
- **THEN** only the failed tests are in the file

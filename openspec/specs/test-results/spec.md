# test-results Specification

## Purpose
Lets users bring execution results from their own test runs into TestModeller, so that coverage can show which covered behavior actually passes.

## Requirements

### Requirement: Import results in standard formats
The system SHALL accept JUnit XML and Cucumber JSON result files for a project and record one result per matched test case.

#### Scenario: JUnit import
- **WHEN** the user imports a valid JUnit XML file
- **THEN** each test result that matches a test case is recorded with its status, duration and failure message

#### Scenario: Cucumber import
- **WHEN** the user imports a valid Cucumber JSON file
- **THEN** each scenario that matches a test case is recorded, with the scenario failing if any step failed

#### Scenario: Invalid or oversized file
- **WHEN** the file is malformed, in an unknown format, or larger than the allowed size
- **THEN** the import is rejected with an explanation and nothing is recorded

### Requirement: Matching results to test cases
The importer SHALL match a result to a test case by the stable test case tag from the exports, and otherwise by exact name within the project, and SHALL report every result it could not match.

#### Scenario: Match by tag
- **WHEN** a result carries the tag of an existing test case
- **THEN** it is recorded for that test case regardless of its name

#### Scenario: Match by name
- **WHEN** a result has no tag but its name equals exactly one test case's name
- **THEN** it is recorded for that test case

#### Scenario: Ambiguous or unknown
- **WHEN** a result's name matches several test cases, or none
- **THEN** it is not recorded and appears in the import report as unmatched or ambiguous

### Requirement: Exports carry a stable test case tag
Gherkin and CSV exports SHALL include a tag that identifies each test case independently of its name and position.

#### Scenario: Gherkin export
- **WHEN** test cases are exported as Gherkin
- **THEN** each scenario carries a tag that the importer recognises

### Requirement: Import report
Each import SHALL return a summary of how many results were matched, recorded, unmatched and ambiguous, with the unmatched names.

#### Scenario: Partial match
- **WHEN** 40 of 45 results match
- **THEN** the report says 40 recorded and lists the 5 unmatched names

### Requirement: Results history and latest result
The system SHALL keep each imported result with its run and time, and SHALL treat the most recent result by execution time as a test case's latest result.

#### Scenario: Re-import
- **WHEN** a newer run is imported for a test case
- **THEN** the older result stays in its history and the new one becomes the latest

#### Scenario: Same run twice
- **WHEN** the same file is imported twice
- **THEN** results are not duplicated

### Requirement: Results are visible where tests are
The latest result SHALL be shown on test cases, in lists and on state chips, not by colour alone, and coverage SHALL be able to count how many covered elements have only passing tests.

#### Scenario: Failing test
- **WHEN** a test case's latest result is failed
- **THEN** it is marked failed in lists and on its state's chips, with a text label

#### Scenario: Passing coverage
- **WHEN** coverage is shown for a model with results
- **THEN** it can show how many covered states and transitions are passing

### Requirement: Tenant isolation
Results SHALL be stored and read only within the caller's tenant, and an import SHALL only affect test cases of the project in the request.

#### Scenario: Other tenant's test case
- **WHEN** a result's tag names a test case in another tenant
- **THEN** it is reported unmatched and nothing is recorded for that tenant

# traceability Specification

## Purpose
Answers which tests, model elements and implementations back each backlog item, and where traceability is missing, for QA leads and developers.

## Requirements

### Requirement: Links are first-class on test cases
A test case SHALL have an optional implementation link and an optional backlog item link, each a full http(s) URL, stored as their own fields and returned by the API.

#### Scenario: Save links
- **WHEN** a user saves a test case with both links
- **THEN** both are returned as separate fields and not as part of the description

#### Scenario: Invalid link
- **WHEN** a link is not a valid http(s) URL
- **THEN** the save is rejected with a validation error naming the field

#### Scenario: Existing data
- **WHEN** a test case holds links written into its description by earlier versions
- **THEN** the links are moved to the fields and the description no longer contains those lines

### Requirement: Backlog item is the unit of traceability
The system SHALL group test cases by the backlog item URL they link to, and a backlog item SHALL be traced to its test cases, the states and transitions they are assigned to, the models, features and components of those, and each test case's implementation link.

#### Scenario: Trace a backlog item
- **WHEN** the user opens the trace for a backlog item
- **THEN** it lists each linked test case with its feature, component, assigned model elements and implementation link

#### Scenario: Several test cases
- **WHEN** one backlog item is linked by several test cases
- **THEN** all appear under that single item

#### Scenario: Different URLs, same item
- **WHEN** two test cases link URLs that differ only by trailing slash, fragment or letter case of the host
- **THEN** they are grouped as one backlog item

### Requirement: Traceability gaps
The system SHALL identify test cases without a backlog link, test cases without an implementation link, and backlog items whose test cases cover no model element.

#### Scenario: Untraced tests
- **WHEN** a test case has no backlog link
- **THEN** it appears in the gaps view as untraced

#### Scenario: Unimplemented
- **WHEN** a test case has a backlog link but no implementation link
- **THEN** its backlog item shows it as not implemented

#### Scenario: Linked to nothing
- **WHEN** all of a backlog item's test cases are assigned to no state or transition
- **THEN** the item is flagged as covering no model element

### Requirement: Traceability matrix view
The UI SHALL provide a traceability page showing backlog items as rows, filterable by component, feature and gap type, searchable by URL or test case name, and exportable as CSV.

#### Scenario: Filter by gap
- **WHEN** the user filters for "no implementation"
- **THEN** only backlog items with such test cases are shown

#### Scenario: Export
- **WHEN** the user exports the matrix
- **THEN** a CSV with one row per backlog item and test case pair is downloaded

#### Scenario: Open a row
- **WHEN** the user opens a test case in the matrix
- **THEN** it opens in the test case editor, and links open in a new tab safely

### Requirement: Tenant and project scope
Traceability SHALL include only data of the caller's tenant and the requested project.

#### Scenario: Other tenant
- **WHEN** another tenant has test cases with the same backlog URL
- **THEN** they do not appear

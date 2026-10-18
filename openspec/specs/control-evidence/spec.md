# control-evidence Specification

## Purpose
Lets a control or a security test result point at evidence, and makes "verified" mean verified: a control cannot be marked verified without evidence, and verification has a review date.

## Requirements

### Requirement: Evidence records
A control and a security test result SHALL each accept any number of evidence records with a kind (document, test-report, scan, configuration, review, other), a title, a link or a note (at least one), the date of the evidence and the name of its reviewer. Evidence SHALL be listed with the control or result it belongs to and SHALL be deletable. Evidence stores references only; the system SHALL NOT store files.

#### Scenario: Add evidence
- **WHEN** the user adds evidence of kind "test-report" with a link to a control
- **THEN** it appears in the control's evidence list with its date and reviewer

#### Scenario: Evidence needs a reference
- **WHEN** the user saves evidence with neither link nor note
- **THEN** the save is refused

### Requirement: Verified requires evidence
A control SHALL be set to status "verified" only when it has at least one evidence record; the verification date SHALL be set to the date of the change and the user MAY enter a review-by date. Removing the last evidence of a verified control SHALL set it back to "implemented".

#### Scenario: Verify without evidence
- **WHEN** the user sets a control with no evidence to "verified"
- **THEN** the change is refused with a message that evidence is required

#### Scenario: Verify with evidence
- **WHEN** the user sets a control with one evidence record to "verified"
- **THEN** the status is verified and the verification date is today

#### Scenario: Last evidence removed
- **WHEN** the user deletes the only evidence of a verified control
- **THEN** the control becomes "implemented" and the user is told why

### Requirement: Overdue verification
A verified control whose review-by date is before today (UTC) SHALL carry the flag `verification-overdue`; the Overview SHALL count such controls. The flag SHALL NOT change the control's status or any rating.

#### Scenario: Overdue
- **WHEN** a verified control has review-by yesterday
- **THEN** it shows `verification-overdue` and the Overview counts it

#### Scenario: Not overdue
- **WHEN** a verified control has no review-by date
- **THEN** it shows no overdue flag

# Spec Delta

## Purpose

Makes risk acceptance a recorded, time-limited decision: who accepted, when, until when, and what happens when it lapses.

## ADDED Requirements

### Requirement: Acceptance record
Treatment "accept" SHALL require a rationale, the approver's name and an expiry date in the future (at most 3 years ahead), and SHALL record the acceptance date as today. The acceptance record SHALL be shown on the threat and in the register. Accepting without an expiry SHALL be refused.

#### Scenario: Accept with expiry
- **WHEN** the user accepts a threat with rationale, approver "J. Doe" and expiry in 12 months
- **THEN** the threat is accepted and shows approver, date and expiry

#### Scenario: Missing expiry
- **WHEN** the user accepts a threat without an expiry date
- **THEN** the change is refused

#### Scenario: Expiry in the past
- **WHEN** the user enters an expiry date before today
- **THEN** the change is refused

### Requirement: Expired acceptance
An accepted threat whose expiry date is before today (UTC) SHALL be treated as open with the flag `acceptance-expired` on read; the stored acceptance record SHALL be kept. Re-accepting with a new expiry SHALL clear the flag. The system SHALL NOT silently delete or extend an acceptance.

#### Scenario: Lapse
- **WHEN** an accepted threat's expiry date passes
- **THEN** the threat appears as open, flagged `acceptance-expired`, with its earlier rationale and approver visible

#### Scenario: Renew
- **WHEN** the user re-accepts it with a new rationale and an expiry in 6 months
- **THEN** the threat is accepted again and the flag disappears

### Requirement: Expiring acceptances
The Overview SHALL list accepted threats expiring within 30 days and the count of expired ones.

#### Scenario: Expiring soon
- **WHEN** an acceptance expires in 10 days
- **THEN** it is listed under "expiring within 30 days"

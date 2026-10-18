# threat-risk-assessment Specification

## Purpose
Rates how serious each threat is on a 5 × 5 likelihood-by-impact matrix, before and with existing controls, classifies the score with configurable thresholds and records how the risk is treated and by whom.

## Requirements

### Requirement: Likelihood and impact ratings
Each threat SHALL carry an inherent rating and a current rating, each a likelihood (1 rare … 5 almost certain) and an impact (1 negligible … 5 severe), both optional until assessed. The score of a rating SHALL be likelihood × impact (1–25). An unassessed rating SHALL show "Not assessed", never a default score.

#### Scenario: Score
- **WHEN** a threat's inherent likelihood is 4 and impact is 5
- **THEN** its inherent score is 20

#### Scenario: Not assessed
- **WHEN** a threat has no current rating
- **THEN** the threat shows "Not assessed" for current risk and is not counted in any matrix cell for current risk

#### Scenario: Out of range
- **WHEN** a client sends a likelihood of 6
- **THEN** the request is rejected as invalid

### Requirement: Risk level thresholds
Each threat model SHALL have thresholds mapping scores to levels Low, Moderate, High and Critical, defaulting to Low 1–4, Moderate 5–9, High 10–16, Critical 17–25. The user SHALL be able to change the thresholds; they MUST cover 1–25 contiguously without overlap. A change SHALL reclassify every threat at once.

#### Scenario: Default levels
- **WHEN** a new threat model rates a threat 3 × 4
- **THEN** its level is High

#### Scenario: Custom thresholds
- **WHEN** the user sets High to 10–14 and Critical to 15–25
- **THEN** a threat scored 15 is shown as Critical

#### Scenario: Gap rejected
- **WHEN** the user sets Moderate to 5–8 and High to 10–16
- **THEN** the change is rejected because score 9 has no level

### Requirement: Current risk is the analyst's judgement
The current rating SHALL be entered by the user. The system SHALL NOT lower a current or inherent rating because a control is linked or its status changes. When a current rating is lower than the inherent one and no linked control is implemented or verified, the threat SHALL carry the warning "current risk lower than inherent without an implemented control".

#### Scenario: Linking a control changes nothing
- **WHEN** the user links a verified control to a threat rated 4 × 5
- **THEN** both ratings stay 4 × 5

#### Scenario: Unsupported reduction
- **WHEN** a threat's inherent score is 20, current score 6 and its only control is planned
- **THEN** the threat shows the warning about an unsupported reduction

### Requirement: Risk treatment
Each open threat SHALL have a treatment (undecided, mitigate, avoid, transfer, accept), a risk owner and an optional due date. Treatment "accept" SHALL require a rationale and the name of the person accepting, and SHALL set the threat's status to accepted; changing the treatment away from "accept" SHALL return it to open.

#### Scenario: Accept a risk
- **WHEN** the user sets treatment "accept" with a rationale and the approver "J. Doe"
- **THEN** the threat's status becomes accepted and the rationale and approver are shown

#### Scenario: Accept without rationale
- **WHEN** the user sets treatment "accept" without a rationale
- **THEN** the change is refused

#### Scenario: Overdue
- **WHEN** an open threat with treatment "mitigate" has a due date in the past
- **THEN** it is marked "Overdue" on the Risks tab and counted on the Overview

### Requirement: Risk register and matrix
The Risks tab SHALL show a 5 × 5 matrix with the number of open and accepted threats per cell, switchable between inherent and current rating, and a register listing each assessed threat with inherent and current score and level, treatment, owner and due date. Selecting a matrix cell SHALL filter the register to its threats. The register SHALL be exportable as CSV.

#### Scenario: Cell filter
- **WHEN** the user activates the cell likelihood 4, impact 5 in the current view
- **THEN** the register lists only threats whose current rating is 4 × 5

#### Scenario: Matrix is accessible
- **WHEN** a screen reader user moves through the matrix
- **THEN** each cell announces its likelihood, impact, level and count

#### Scenario: CSV export
- **WHEN** the user exports the register
- **THEN** a CSV with one row per assessed threat and the register's columns is downloaded

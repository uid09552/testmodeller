# residual-risk Specification

## Purpose
Adds the residual rating — expected risk after planned measures are implemented and verified — alongside inherent and current risk, so a threat model can state what risk is left.

## Requirements

### Requirement: Residual rating
Each threat SHALL carry an optional residual rating (likelihood 1–5, impact 1–5) entered by the user, scored and classified with the same formula and thresholds as the inherent and current ratings. An unassessed residual rating SHALL show "Not assessed", never a default score. The system SHALL NOT derive or change a residual rating from controls.

#### Scenario: Residual score and level
- **WHEN** a threat's residual likelihood is 2 and impact is 3
- **THEN** its residual score is 6 and its level is Moderate under the default thresholds

#### Scenario: Linking a control changes nothing
- **WHEN** the user links a verified control to a threat with residual rating 3 × 3
- **THEN** the residual rating stays 3 × 3

#### Scenario: Out of range
- **WHEN** a client sends a residual impact of 0
- **THEN** the request is rejected as invalid

### Requirement: Residual flags
A threat SHALL carry the flag `residual-above-current` when its residual score is higher than its current score, and the flag `unsupported-residual` when its residual score is lower than its current score and none of its linked controls is verified. Both flags SHALL be computed on read and SHALL list the facts that triggered them.

#### Scenario: Residual above current
- **WHEN** a threat has current score 6 and residual score 9
- **THEN** the threat shows `residual-above-current`

#### Scenario: Unsupported residual
- **WHEN** a threat has current score 12, residual score 4 and only implemented (not verified) controls
- **THEN** the threat shows `unsupported-residual` naming those controls

#### Scenario: Supported residual
- **WHEN** the same threat has a verified control with evidence
- **THEN** the threat does not show `unsupported-residual`

### Requirement: Residual risk in matrix and register
The risk matrix SHALL offer a view selector (inherent, current, residual) and the risk register SHALL show all three scores and levels per threat, with sorting and filtering by residual level. Threats without a residual rating SHALL NOT be counted in any residual matrix cell.

#### Scenario: Switch matrix view
- **WHEN** the user selects "residual" in the matrix
- **THEN** each cell counts the threats whose residual rating falls in it

#### Scenario: Register columns
- **WHEN** the user opens the risk register
- **THEN** inherent, current and residual score and level are shown, and "Not assessed" appears where unrated

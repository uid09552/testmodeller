# Spec Delta

## Purpose

Produces the deliverable of a threat and risk analysis — a readable report — and an indicator of how complete the analysis is.

## ADDED Requirements

### Requirement: Assessment report
The user SHALL be able to open a report for the live threat model or for a baseline. The report SHALL contain, in this order: title, date and baseline name if any; scope, objectives, assumptions and exclusions; architecture summary (zones, components, flows, trust-boundary crossings); assets; risk matrix per view (inherent, current, residual); risk register with treatment, owner, due date and acceptance record; controls with status and evidence; open flags and warnings; and a sign-off block with empty name, role and date lines.

#### Scenario: Open report
- **WHEN** the user opens the report of the live model
- **THEN** all listed sections are shown with current data

#### Scenario: Baseline report
- **WHEN** the user opens the report of baseline "Release 2.3 review"
- **THEN** the report shows the baseline's data and names the baseline and its date

### Requirement: Report export
The report SHALL be printable with a print stylesheet that hides application chrome and avoids splitting table rows across pages, and exportable as Markdown. Generated report text SHALL be deterministic, not AI output.

#### Scenario: Print
- **WHEN** the user chooses print
- **THEN** only the report content is printed, with section headings starting new pages where sensible

#### Scenario: Markdown
- **WHEN** the user exports Markdown
- **THEN** a file with the same sections in the same order is downloaded

### Requirement: Completeness indicator
The Overview SHALL show the number of open completeness findings, each linking to the entity: components without owner or zone, flows with unknown authentication or encryption, threats without inherent rating or treatment, open threats without owner, controls verified without review-by date, and expired or expiring acceptances. The indicator SHALL not be a single score.

#### Scenario: Findings listed
- **WHEN** two components have no owner and one open threat has no treatment
- **THEN** the Overview lists those three findings with links

#### Scenario: Clean model
- **WHEN** no finding applies
- **THEN** the Overview shows "No open findings"

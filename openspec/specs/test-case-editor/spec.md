# test-case-editor Specification

## Purpose
Defines the behavior of the overlay used to edit a single Gherkin test case, so that users do not lose unsaved edits through accidental interaction.

## Requirements

### Requirement: Overlay closes only on explicit action
The test case overlay SHALL close only when the user activates Save, Cancel, or the close button in its top-right corner. No other interaction SHALL dismiss it.

#### Scenario: Click on the backdrop
- **WHEN** the user clicks the dimmed area outside the dialog
- **THEN** the overlay remains open and its draft is unchanged

#### Scenario: Drag from inside to outside
- **WHEN** the user presses the pointer inside a field (e.g. to select text) and releases it outside the dialog
- **THEN** the overlay remains open and its draft is unchanged

#### Scenario: Escape key
- **WHEN** the user presses Escape while the overlay is open
- **THEN** the overlay remains open

#### Scenario: Cancel and close button
- **WHEN** the user activates Cancel or the top-right close button
- **THEN** the overlay closes and the draft is discarded

#### Scenario: Save
- **WHEN** the user activates Save with a valid draft
- **THEN** the draft is saved and the overlay closes

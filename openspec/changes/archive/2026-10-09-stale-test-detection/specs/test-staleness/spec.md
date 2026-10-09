# Spec Delta

## Purpose

Tells users which generated test cases no longer match the model they were generated from, and why, so that they can act before the tests mislead.

## ADDED Requirements

### Requirement: Staleness is derived from the current model
The system SHALL determine staleness of a generated test case by checking its path against the current model, and SHALL NOT require a stored flag or a regeneration to do so.

#### Scenario: Unchanged model
- **WHEN** a generated test case's path still exists and is feasible
- **THEN** it is not reported stale

#### Scenario: Manual and AI tests
- **WHEN** a test case was created manually or by AI
- **THEN** it is not checked for path staleness

### Requirement: Reasons for staleness
A stale test case SHALL be reported with one or more reasons: a step lost its assignment, consecutive steps no longer connect, the path no longer starts at the initial state, or a guard on the path can no longer be satisfied.

#### Scenario: Element deleted
- **WHEN** a transition used by a generated test case is deleted
- **THEN** the test case is reported stale with reason "step lost its assignment"

#### Scenario: Transition rewired
- **WHEN** a transition's target is changed so two consecutive steps no longer connect
- **THEN** the test case is reported stale with reason "steps no longer connect"

#### Scenario: Guard becomes unsatisfiable
- **WHEN** a guard is edited so that no input can satisfy the path
- **THEN** the test case is reported stale with reason "guard unsatisfiable"

#### Scenario: Initial state moved
- **WHEN** the initial state changes and the path starts elsewhere
- **THEN** the test case is reported stale with reason "does not start at the initial state"

### Requirement: Staleness is visible where tests are
The editor and the test case lists SHALL mark stale tests and show their reasons, and the model SHALL show how many of its tests are stale.

#### Scenario: Marker with reason
- **WHEN** a stale test case is shown in a list or on a state's chips
- **THEN** it carries a visible marker and its reasons, not conveyed by colour alone

#### Scenario: Model count
- **WHEN** a model has stale tests
- **THEN** the Test Cases tab shows the count and the Validation tab lists them as warnings

### Requirement: Stale tests are never changed automatically
Detecting staleness SHALL NOT modify, unassign or delete any test case.

#### Scenario: Reload
- **WHEN** staleness is recomputed
- **THEN** no test case or assignment is altered

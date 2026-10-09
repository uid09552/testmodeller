# Spec Delta

## Purpose

Lets users step through a model's behaviour interactively, with guards and actions evaluated exactly as test generation evaluates them, to check a model before generating tests from it.

## ADDED Requirements

### Requirement: Start a simulation
Starting a simulation SHALL put the model in its initial state with every variable at its initial value, and SHALL simulate the model as currently shown in the editor, including unsaved changes.

#### Scenario: Start
- **WHEN** the user starts a simulation on a valid model
- **THEN** the initial state is marked as current and the variables are listed with their initial values

#### Scenario: Unsaved edit
- **WHEN** the user has just changed a guard and not yet saved
- **THEN** the simulation uses the changed guard

#### Scenario: Model without an initial state
- **WHEN** the model has no initial state or several
- **THEN** the simulation does not start and says why

### Requirement: Enabled and blocked transitions
At each step, the simulation SHALL list every outgoing transition of the current state as enabled or blocked. A blocked transition SHALL show the guard that is false, or the evaluation error.

#### Scenario: Guard blocks
- **WHEN** the current state has a transition with guard `attempts < 2` and `attempts` is 2
- **THEN** that transition is listed as blocked with its guard, and is marked on the canvas as not available

#### Scenario: Dead end
- **WHEN** no transition is enabled and the state is not final
- **THEN** the simulation says the model is stuck in that state

### Requirement: Take, undo and record steps
Taking an enabled transition SHALL apply its action, move to its target state and add it to the trail. The user SHALL be able to step back, restart, and save the trail as a new manual test case of the model's feature, assigned to the transitions and states taken in order.

#### Scenario: Take a transition
- **WHEN** the user takes an enabled transition whose action is `attempts = attempts + 1`
- **THEN** the current state becomes its target and `attempts` increases by one

#### Scenario: Step back
- **WHEN** the user steps back
- **THEN** the previous state and variable values are restored

#### Scenario: Save as test case
- **WHEN** the user saves a trail of three steps
- **THEN** a manual test case with three steps (event as action, target state as expected result) is created and assigned to those transitions and states in step order

### Requirement: Same semantics as generation
A simulation step SHALL evaluate guards and actions with the same expression language and the same rules as test generation.

#### Scenario: Agreement with generation
- **WHEN** generation produced a path for a model
- **THEN** taking the same transitions in a simulation is possible at every step and ends in the same state

---
title: Domain model
type: specification
status: draft
tags: [domain, entities]
related: [01-glossary.md, 04-test-generation.md, 05-api.md]
---

# Domain model

```mermaid
erDiagram
  PROJECT ||--o{ COMPONENT : has
  COMPONENT ||--o{ FEATURE : has
  FEATURE ||--o{ MODEL : has
  FEATURE ||--o{ TEST_CASE : owns
  MODEL ||--o{ STATE : contains
  MODEL ||--o{ TRANSITION : contains
  TEST_CASE ||--o{ TEST_STEP : has
  TEST_CASE }o--o{ STATE : assigned_to
  TEST_CASE }o--o{ TRANSITION : assigned_to
  PROJECT ||--o{ PROPOSAL : has
```

## Entities

All entities: `id` (UUID), `name`, `description`, `createdAt`, `updatedAt`, `version`.

- **Project**
- **Component**: `projectId`
- **Feature**: `componentId`, `scenarioDescription` (markdown text describing the scenarios), `tags[]`
- **Model**: `featureId`, `variables[]`, `status` (draft|ready); a state machine formerly called "scenario"
- **State**: `modelId`, `kind` (normal|initial|final), `position {x,y}`
- **Transition** (model step): `modelId`, `from`, `to`, `event`, `guard?`, `action?`, `expected?`
- **TestCase**: `featureId`, `steps[]`, `status`, `priority`, `tags[]`, `origin` (manual|generated|ai), `generatedFromModelId?`
- **TestStep**: `order`, `action`, `expected`
- **Assignment**: links a TestCase to a State or Transition (optionally a test step) of a Model of the same feature
- **Proposal**: `kind`, `payload` (JSON), `status` (pending|accepted|rejected), `rationale`, `source` (LLM id)

## Invariants
- A model has exactly one initial state.
- Transitions reference states of the same model.
- A test case may be assigned to many states/transitions, only within models of its own feature.
- Deleting a model or state/transition removes its assignments, never the test cases.
- Deleting a component/feature requires it to be empty or cascades explicitly.
- Guards/actions use the expression language defined in `04-test-generation.md`.

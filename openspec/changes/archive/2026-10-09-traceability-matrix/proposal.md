# Proposal

## Why

The QA lead's question, "is requirement X covered, and by what?", has no single answer in the tool. Test cases can hold an implementation link and a backlog link, but those are stored as labelled lines inside the free-text `description` (the API has no fields for them), so nothing can query them. Without that, a backlog item cannot be traced to its tests, model elements and implementation.

## What Changes

- Give test cases real `implementationUrl` and `backlogUrl` fields in the API and database, migrating the values currently embedded in descriptions.
- A read-only traceability endpoint for a project: backlog item → test cases → model elements (state or transition, model, feature, component) → implementation links, plus the gaps.
- A Traceability page in the UI: a matrix grouped by backlog item with filters, a gaps view (backlog items without tests, tests without a backlog item, tests without an implementation), and CSV export.
- "Requirement" means a **backlog item**, identified by its URL. No new requirement entity is introduced.
- **API changes (need your approval before they are applied):** the two fields on `TestCase`/`TestCaseInput`, and `GET /projects/{projectId}/traceability`. A database migration is required.

## Capabilities

### New Capabilities
- `traceability`: Tracing backlog items to test cases, model elements and implementations, and finding the gaps.

### Modified Capabilities

## Impact

- `backend/crates/storage` (migration `0005` or next free number: new columns plus backfill from descriptions), `api` (fields, new route), `domain`.
- `frontend/src/features/models/state/model-mapping.ts` stops packing links into `description`; a new Traceability feature and route.
- `openapi.yaml`, `05-api.md`, `02-domain-model.md`, `03-requirements.md`, `06-ui.md`.
- Works without `test-results-import`; if results exist, the matrix shows the latest result per test case.

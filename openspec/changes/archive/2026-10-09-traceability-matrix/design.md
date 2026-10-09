# Design

## Context

The editor packs the two links into the test case `description` as `Implementation: <url>` / `Backlog: <url>` lines and parses them back (`model-mapping.ts`: `IMPL`, `BACKLOG`), because the API has no fields. That makes them unqueryable and fragile (a user editing the description can break them). Assignments connect test cases to states/transitions of models of the same feature. The Test Cases page currently loads and joins everything in the browser. See proposal.md.

## Goals / Non-Goals

**Goals:**
- Real fields, one server-side query for the whole trace.
- Backlog URL as the only notion of "requirement".

**Non-Goals:**
- A requirement entity with ids, text or hierarchy.
- Syncing with Jira or GitHub (status, titles). A later change can enrich the item by URL.
- Computing coverage; that stays in the coverage feature.

## Decisions

- **Add columns `implementation_url` and `backlog_url`** to test cases (nullable, validated http(s), max 2048 chars), plus `implementationUrl`/`backlogUrl` in `TestCase` and `TestCaseInput`. Backfill by parsing the labelled description lines and stripping them; the migration is idempotent and leaves descriptions without those lines untouched. Alternative: parse descriptions on the server — rejected: perpetuates the hack.
- **Normalised key**: lower-case scheme and host, drop fragment and trailing slash, keep query. Stored as `backlog_key` (computed on write) to index grouping. The raw URL stays what the user typed.
- **One endpoint** `GET /projects/{projectId}/traceability?componentId&featureId&gap=` returning `{ items: [{ backlogUrl, testCases: [{ id, name, featureId, componentId, implementationUrl, elements: [{ modelId, stateId|transitionId }], lastResult? }] }], untraced: [...], summary }`. Alternative: more client-side joins — rejected: the page would make many calls and repeat the logic that the CSV export needs.
- **Gap rules** live in the backend so UI, export and tests share them.
- **Frontend**: new `features/traceability` with a table, filters, a gaps tab and CSV export (client-side from the response); `model-mapping.ts` reads and writes the new fields and keeps reading the old description lines only as a fallback for unmigrated local data.
- **Display**: URLs shown through `linkLabel`; external links via `safeExternalUrl` with `rel="noopener noreferrer"` as elsewhere.

## Risks / Trade-offs

- [Migration mis-parses an unusual description] → only exact `Implementation: ` / `Backlog: ` line prefixes are moved; a test run on a copy of real data first; migration is reversible by re-appending the lines (down script).
- [Local-only models saved before the change] → mapping fallback reads old lines and writes the new fields on next save.
- [Large projects] → the query is one pass over test cases with assignments; page and filter server-side by component and feature.
- [Same item written with different URLs (e.g. issue key vs full URL)] → out of scope; documented.

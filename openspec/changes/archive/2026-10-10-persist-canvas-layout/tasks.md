# Tasks

## 1. Contract (requires approval)

- [x] 1.1 Get explicit approval for the optional `layout` object on `Model` and `ModelInput` (JSON object, max 256 KB, absent keeps the stored value, `null` clears it); then update `openapi.yaml`, `05-api.md` and `02-domain-model.md`; verify the spec parses and Redocly shows no new problem kinds
- [x] 1.2 Write the ADR (follow-up to 0008: layout is opaque, owned by the frontend, versioned with `v`); verify it is listed in `docs/adr/README.md`

## 2. Backend

- [x] 2.1 Add migration `0007` (`models.layout jsonb`) and carry layout through load, replace (absent keeps it, `null` clears it), create, version snapshots and restore; verify storage tests on a fresh database and on one with existing models (layout absent)
- [x] 2.2 Validate layout in the api crate (object only, at most 256 KB, error names `layout`) and leave the stored model unchanged on rejection; verify API tests for a valid, an oversized and a non-object layout
- [x] 2.3 Remap element ids inside layout on duplicate and on JSON import, and include layout in JSON export; verify tests that a duplicate's layout keys point at the copy's states and transitions, and that export then import keeps layout
- [x] 2.4 Verify layout does not affect behaviour: generation, validation and coverage of the same graph with and without layout give identical results (test)

## 3. Frontend

- [x] 3.1 Carry `variables` through `fromRemote` / `toModelInput` unchanged; verify a mapping test that a model with variables round-trips, and a persistence test that a save after a move sends the variables
- [x] 3.2 Write and read layout v1 in `model-mapping.ts` (state shape, colour, size, decision kind; groups; transition curve and anchors; test display numbers; only non-default values; entries for missing elements dropped); verify mapping tests for round trip, missing layout, partial layout and stale entries
- [x] 3.3 Include layout in `remoteFingerprint` so layout-only edits are saved; verify a persistence test that a colour change schedules a save and a reload restores it
- [x] 3.4 Describe what the editor stores in `06-ui.md` and update the comments in `model-mapping.ts` and `model-persistence.ts` that say presentation is not stored; run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `ng lint`, `ng test` and verify all pass

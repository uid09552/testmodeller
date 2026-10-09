# TestModeller: feature candidates

Ideas for what to consider next, ranked by value for the people the tool serves (test engineer, QA lead, developer) and grounded in what the specification and code already have. This is a discussion list, not a commitment. Anything adopted should go through the normal flow: spec in `docs/specification/`, then an OpenSpec change.

Effort: **S** days, **M** one to two weeks, **L** more than two weeks. Impact is a judgement call.

## 1. Close the loop between model and tests (highest value)

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **Traceability matrix**: requirement or backlog item → feature → model element → test case → implementation link | Test cases already carry `implementationUrl` and `backlogUrl`. A matrix answers "is requirement X covered, and by what?" for the QA lead. | High | M |
| **Test execution results import** (JUnit XML, Cucumber JSON) mapped to test case ids | Execution runners are out of scope for v1, but *reading* results is not execution. It turns coverage from "modelled" into "passing". | High | M |
| **Coverage gaps on the canvas**: colour states and transitions the current test cases do not reach, per chosen criterion | The canvas already shows test chips per state. Showing what is *uncovered* is the natural next signal. Builds on FR-022. | High | S–M |
| **Stale-test detection**: when a model changes, flag test cases whose path no longer exists or whose guard became unsatisfiable | Regenerate-and-diff (FR-023) helps only if someone remembers to regenerate. | High | M |

## 2. Better models

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **Hierarchical (nested) states and sub-models** | Real state machines outgrow one canvas. The spec's NFR-001 already assumes 200-state models, which are unreadable flat. | High | L |
| **Model variables panel** with types, ranges and initial values | Guards and actions use variables today with no declared home. A panel enables autocomplete and earlier validation errors. | Medium | M |
| **Guard and action expression editor** with syntax checking, completion and "why infeasible" hints | The expression language is non-trivial; errors currently surface at generation time. | Medium | M |
| **Model compare**: visual diff between two versions (FR-014 gives the versions) | Review of model changes is otherwise a guess. | Medium | M |
| **Templates and patterns**: login, wizard, CRUD, retry-with-timeout as insertable sub-graphs | Speeds up first models and nudges toward good structure. | Medium | S–M |
| **Import from other formats**: PlantUML, Mermaid state diagrams, SCXML, BPMN | Teams already have diagrams; re-drawing blocks adoption. | Medium | M |

## 3. Smarter generation

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **Test data per step**: boundary values and equivalence classes for variables | Paths say *what flow*; testers also need *which inputs*. This is the usual next layer in model-based testing. | High | L |
| **Risk-weighted generation**: weights or priorities on states and transitions, with a "cover high risk first" ordering and a size budget | "Transition-pair" on a big model yields too many tests; teams want the best N. | High | M |
| **Suite minimisation reports**: show which tests are redundant for a chosen criterion | Lets people trim a manual regression suite with evidence. | Medium | S |
| **Additional criteria**: all-round-trips, loops up to k, MC/DC-style guard coverage | Guard-aware criteria are the stronger option for the expression language. | Medium | M |
| **Negative and robustness generation**: events that are *not* allowed in a state | The model implies these; the AI proposal path covers them only by suggestion. | Medium | M |

## 4. AI that earns trust

All proposals stay reviewable and are never persisted unapproved (project rule 8).

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **Model critique**: AI reviews a model against its scenario description and lists missing behaviours, ambiguities and contradictions | Cheap, review-only, no data written. | High | S–M |
| **Requirement → model with source citations**: each proposed state links to the sentence it came from | Makes review fast and proposals auditable (FR-034). | Medium | M |
| **Explain a test case**: plain-language reason a generated path exists and what it covers | Helps developers consuming exports. | Medium | S |
| **Provider and cost visibility**: token use per proposal, per tenant | Needed once more than one team uses the AI. | Low–Medium | S |

## 5. Team and workflow

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **Review workflow for models and tests** (draft → in review → approved, with comments) | Status values exist; the process around them does not. | Medium–High | M |
| **Comments and @mentions on states, transitions and test cases** | Replaces review chatter in side channels. Not real-time co-editing, which is out of scope. | Medium | M |
| **Audit trail**: who changed what, tenant-wide | Follows from the tenancy work in FR-044; useful for regulated users. | Medium | M |
| **Notifications** (webhook or email) on proposal ready, model approved, tests regenerated | Low-cost glue for CI and chat tools. | Low–Medium | S |
| **Roles beyond User/Editor**: reviewer, project-scoped permissions | Today a tenant-wide editor can change anything. | Medium | M |

## 6. Integration and delivery

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **CI-friendly CLI and API token**: `tm export --feature X --format gherkin` to a repo, fail the build if the model is invalid or tests are stale | Makes the tool part of the pipeline instead of a side site. | High | M |
| **Two-way link with trackers** (Jira, GitHub issues): create or sync items from `backlogUrl`, show status on the test case | Links exist as plain URLs today. | Medium | M |
| **More export targets**: Xray, TestRail, Playwright/Cypress skeletons, pytest-bdd | Gherkin/JSON/CSV cover reading, not adoption in a team's test tool. | Medium | M per target |
| **Repository sync**: commit exported feature files to Git on approval | Developers get tests where they work. | Medium | M |

## 7. Editor experience

| Feature | Why | Impact | Effort |
| --- | --- | --- | --- |
| **Keyboard-first editing and the table view** (NFR-004's alternative to the canvas) | Required for WCAG, and fast for power users. Confirm what exists. | Medium | M |
| **Auto-layout** (layered, with edge routing) and "tidy selection" | Beyond Align; big models become readable in one click. | Medium | M |
| **Search and jump to state**, minimap, and collapse of groups | Navigation for large models. | Medium | S–M |
| **Presentation/read-only mode and shareable view links** | Review sessions with non-editors. | Low–Medium | S |
| **Simulation / step-through** of a model: click through events, watch variables and the current state | Lets testers sanity-check a model before generating, and finds guard bugs. | High | M |

## Suggested order

Reordered after the product-owner review on 2026-10-09 (see the answers below): the primary buyer is the QA lead, and real models are usually larger than 30 states.

1. **Section 1, the model-to-test loop.** All four are proposed as OpenSpec changes: [traceability matrix](openspec/changes/traceability-matrix/proposal.md), [test results import](openspec/changes/test-results-import/proposal.md), [coverage gaps on the canvas](openspec/changes/canvas-coverage-gaps/proposal.md) and [stale-test detection](openspec/changes/stale-test-detection/proposal.md).
2. **Wire up the Coverage dashboard** (it still shows placeholder data, see below) and **CI CLI**: the QA lead's reporting needs real numbers and a pipeline gate.
3. **Hierarchical states**, **search and jump to state**, **auto-layout**: large models are the norm, so navigation and structure come before smaller modelling niceties.
4. **Review workflow** and **audit trail** (section 5): what a QA lead needs to sign off.
5. **Simulation**, **model critique**, **suite minimisation**, then **test data per step**.

## Answers that reordered this list

- Primary buyer: **QA leads who need reporting**. Sections 1 and 5 lead.
- Test management tool: **not in use** by the target teams, so section 6 (exports to Xray/TestRail, tracker sync) stays where it is.
- Model size: **usually more than 30 states**. Hierarchical states and large-model navigation move up.

## Checked against the code (2026-10-09)

| Item | What exists |
| --- | --- |
| Coverage dashboard | Backend endpoints exist (`/models|features|components/{id}/coverage`). The page (`features/coverage/coverage-dashboard.ts`) shows hard-coded placeholder rows and calls no API. |
| Import and export | Backend: JSON, CSV and Gherkin export and JSON import (`handlers/export.rs`). Frontend: no UI for them; the Test Cases page builds its own Gherkin download in the browser. |
| Version history restore | Backend: `GET /models/{id}/versions` and `POST …/versions/{v}/restore`. Frontend: no UI. |
| Table view (NFR-004) | Specified in `06-ui.md` ("Behaviors") but not implemented; the canvas is the only editor. |
| Expression language | Booleans, integers and strings; operators `|| && ! == != < <= > >= + -` and parentheses; no multiplication, division or functions. Generation caps the search at 200,000 nodes and 1,000 bounded paths. Variables exist in the model graph and API but the editor has no way to declare them, which makes the **model variables panel** more urgent than its row suggests. |

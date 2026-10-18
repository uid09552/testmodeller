---
title: TestModeller documentation
type: index
status: active
tags: [index]
related: []
---

# TestModeller

Model-based testing: draw a model of how a feature behaves, and generate the
test cases that cover it. States and transitions go on a canvas, test cases
hang off the states they exercise, and an AI assistant proposes both — nothing
it proposes is saved until you accept it. Alongside the tests, each project can
carry a **threat model**: its architecture, the threats against it, how serious
they are, the controls that answer them, and security tests that check the
controls.

![The model editor, with the project tree in the nav bar and test cases in the right-hand panel](screenshots/02-model-editor.png)

- **Source**: <https://github.com/uid09552/testmodeller>
- **Getting started**: [Running it](#running-it)
- **Concepts**: [Glossary](specification/01-glossary.md) · [Domain model](specification/02-domain-model.md)
- **Threat modeling**: [Tour](#threat-modeling) · [Guide](guides/threat-modeling-guide.md) · [Specification](specification/09-threat-modeling.md)

## A tour

### Projects, components, features, models

The tree lives in the nav bar, so it is there on every screen. A project holds
components, a component holds features, and a feature holds the models that
describe it. Clicking a model opens it in the editor; clicking anything else
shows its detail.

![The project tree in the nav bar, with a feature selected](screenshots/01-projects-tree.png)

### The model editor

The canvas holds states and transitions. Each state carries a chip per test
category — `U`nit, `I`ntegration, `F`eature — with the number of cases in it
and a dot when one of them is negative, so coverage is visible without opening
anything. Double-click the chips to jump to that state's cases.

Select and pan are the two canvas tools (`V` and `H`); double-clicking empty
space centres the diagram.

### Test cases

Test cases are the editor's default right-hand tab. With a state selected it
edits that state's cases in Given/When/Then form; with nothing selected it
shows which states are covered.

![The Test Cases tab, showing the cases on the selected state](screenshots/03-test-cases.png)

### Properties

The Properties tab edits whatever is selected — a state, a transition, a group
or the model itself — including guards and actions on transitions.

![The Properties tab for a selected state](screenshots/04-properties.png)

### The AI assistant

Ask for scenarios, test cases or the states a flow is missing. Each answer
comes back as a proposal card with its rationale and the model that produced
it. Accepting one records the acceptance through the API and then applies it;
rejecting one leaves no trace. Nothing is applied on its own.

![The AI assistant panel in the model editor](screenshots/05-ai-assistant.png)

See [AI integration](specification/07-ai-integration.md) for the providers —
Claude, Ollama or any OpenAI-compatible endpoint — and how keys are handled.

### Test cases across the project

Every case in one table, filterable by category, with its state and its
Gherkin steps.

![The test case list](screenshots/06-test-case-list.png)

### Coverage

What the test cases cover, per component, feature and model, and what they do
not.

![The coverage dashboard](screenshots/07-coverage.png)

The figures in this screenshot are placeholders: the dashboard is not wired to
`GET /coverage` yet. See [Status](#status).

### Settings

Shows the AI provider in effect — Claude, Ollama or any OpenAI-compatible
endpoint — and how to change it. The provider is configured in `.env`, not in
the UI:

```bash
TM_AI_PROVIDER=ollama
TM_AI_MODEL=llama3.1
TM_AI_BASE_URL=http://host.docker.internal:11434/v1   # Ollama on this machine
```

![AI provider settings](screenshots/08-settings.png)

## Threat modeling

Each project can have one threat model, listed in the project tree directly
under the project. It answers five questions — what are we building, what can
go wrong, how serious is it, what do we do about it, and did it work — in a
workspace of tabs. The [threat modeling guide](guides/threat-modeling-guide.md)
walks through it step by step.

### Overview

Counts, threats by status and level, the risk matrix, and what is missing: open
threats without a control, controls without a test, components without an owner
or a zone, and acceptances about to expire. The scope, objectives, assumptions
and exclusions are edited here.

![The threat model overview](screenshots/09-threat-overview.png)

### Architecture: diagram, components, flows, zones, assets

The data-flow diagram draws components inside dashed **exposure zones**, each
with a trust level from 0 (untrusted) to 4. A diamond marks a flow that crosses
a trust boundary. The same data is editable in the Components, Data Flows,
Zones and Assets tables, and anything not yet documented — an owner, how a
crossing flow authenticates, whether it is encrypted — shows as a warning
instead of defaulting to "fine".

![The data-flow diagram of the Webshop](screenshots/10-threat-diagram.png)

![The components table, with owners, zones and warnings](screenshots/11-threat-components.png)

### Threats

Threats are classified by STRIDE. Add them by hand, or let **Suggest threats**
apply a fixed set of rules to the architecture: each match becomes a
*candidate* that says why it was suggested, what is missing to confirm or
dismiss it, and which controls it recommends. Candidates are not findings until
you confirm them.

![The threats register, with manual threats and rule-suggested candidates](screenshots/12-threat-threats.png)

### Risks

Each threat is rated by likelihood × impact as inherent (before controls),
current (with the controls that exist) and residual (once planned measures are
verified). The ratings are yours: linking a control never changes one. A risk
can be accepted only with a rationale, an approver and an expiry date.

![The risk matrix and register](screenshots/13-threat-risks.png)

### Controls and evidence

Controls are planned, implemented or verified. A control can be marked verified
only when it has evidence, a reference to a report, scan, configuration or
review.

![The controls table](screenshots/14-threat-controls.png)

### Security tests

A list of its own, separate from the feature test cases. Drafts can be
generated from threats and controls, results are recorded by hand, and the
list exports as CSV or Gherkin.

![The security tests](screenshots/15-threat-security-tests.png)

### Baselines and the report

A baseline is an immutable snapshot of the threat model, which can be compared
with the live model or another baseline. The report renders either as a threat
and risk assessment that can be printed or exported as Markdown.

![Baselines](screenshots/16-threat-baselines.png)

![The threat and risk assessment report](screenshots/17-threat-report.png)

### The threat model's AI assistant

Off by default for every threat model. Once enabled, it proposes architecture,
threats, ratings, controls and security tests as cards you accept or reject,
and it can never verify a control, accept a risk or delete anything. *Preview
context* shows what would be sent to the provider before it is.

![The AI assistant over the Threats tab](screenshots/18-threat-ai-assistant.png)

See [AI integration](specification/07-ai-integration.md#threat-model-assistant)
for the kinds of proposal and the safeguards.

## Running it

```bash
cp .env.example .env     # set POSTGRES_PASSWORD, TM_JWKS_URL and the OIDC_* values
make up                  # gateway, frontend, backend and postgres
```

The app comes up on <http://localhost:8088>, behind an APISIX gateway that does
the OIDC handshake. For development without an identity provider:

```bash
make dev      # backend against a throwaway database, JWT validation off
make dev-fe   # Angular dev server on :4200, proxied to the backend
```

See [User management](specification/08-usermanagement.md) for what `--dev-mode`
skips and why, and [Configuring authentication](guides/authentication-setup.md)
to connect your identity provider, including where the tenant and role sit in
its tokens.

## Documentation

### Specification
- [00 Overview](specification/00-overview.md)
- [01 Glossary](specification/01-glossary.md)
- [02 Domain model](specification/02-domain-model.md)
- [03 Requirements](specification/03-requirements.md)
- [04 Test generation](specification/04-test-generation.md)
- [05 API](specification/05-api.md) — the contract is
  [openapi.yaml](specification/openapi.yaml)
- [06 UI](specification/06-ui.md)
- [07 AI integration](specification/07-ai-integration.md)
- [08 User management](specification/08-usermanagement.md)
- [09 Threat modeling](specification/09-threat-modeling.md)

### Architecture
- [Architecture overview](architecture/overview.md)
- [Decision records](adr/README.md)

### Guides
- [Configuring authentication](guides/authentication-setup.md)
- [Threat modeling guide](guides/threat-modeling-guide.md)
- [Rust coding guide](guides/rust-coding-guide.md)
- [Angular coding guide](guides/angular-coding-guide.md)
- [Testing guide](guides/testing-guide.md)
- [Workflow](guides/workflow.md)

## Status

The backend implements the specification: the full API, test generation, AI
proposals, threat modeling, JWT validation and tenant separation, with unit tests over the
domain, generation, parsing and authorisation logic.

The frontend reads and writes everything through the API; nothing is kept in
the browser. The remaining gaps:

| Area | State |
| --- | --- |
| Projects tree, model editor, test cases | Stored through the API only (ADR 0008). Shapes, colours, groups and edge curves are saved with the model as an opaque layout (ADR 0010) |
| Threat modeling | Working end to end: architecture and diagram, STRIDE suggestions, risk ratings, controls with evidence, security tests, baselines, report, and the opt-in AI assistant |
| AI assistant | Working. Saves the open model first, then asks (ADR 0008) |
| Settings | Read-only view of the provider configured in `.env` |
| Coverage dashboard, AI proposals list | Placeholder data; not wired to the API |
| Test case list | Real, read through the API |

## Contributing

Read the [workflow guide](guides/workflow.md) first. Two rules matter more than
the rest:

1. **Specification first.** Change the relevant file in `specification/` in the
   same commit as the code.
2. **The API contract is fixed.** `openapi.yaml` is the source of truth and is
   not changed without agreement; both sides implement it as written.

The screenshots on this page are generated, not hand-captured — see
[screenshots/README.md](screenshots/README.md) — so re-run them when the UI
changes rather than letting them drift.

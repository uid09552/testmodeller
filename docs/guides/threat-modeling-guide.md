---
title: Threat modeling guide
type: guide
status: active
tags: [threat-modeling, security, risk, stride, guide]
related: [../specification/09-threat-modeling.md, ../specification/06-ui.md, ../adr/0011-threat-model-entities-and-security-tests.md, ../adr/0012-residual-risk-and-baselines.md]
---

# Threat modeling guide

This guide walks through building a threat model in the UI, from the first
component to a report you can hand to a reviewer. It describes how to use the
feature; the rules behind it (how crossings, scores, flags and suggestions are
computed) are in [09 Threat modeling](../specification/09-threat-modeling.md),
and every screen in detail in [06 UI](../specification/06-ui.md#threat-modeling-fr-060-to-fr-076).

The examples use the *Webshop* project shown in the screenshots: a browser, an
API gateway, a shop API, an identity provider and a customer database.

## The idea in one minute

A project has at most one **threat model**. It answers five questions, and
each has a place in the workspace:

| Question | Where |
| --- | --- |
| What are we building? | Diagram, Components, Data Flows, Zones, Assets |
| What can go wrong? | Threats |
| How serious is it? | Risks |
| What do we do about it? | Controls |
| Did it work? | Security Tests, Baselines, Report |

A **threat** is a potential attack scenario. Its **risk** is how likely and how
damaging that scenario is. A **control** is a measure that prevents, detects or
corrects it. They are separate records on purpose: linking a control never
changes a rating by itself, because only you can judge whether it works.

## 1. Create the threat model

Right-click a project in the tree and choose **New Threat Model**. It appears
directly under the project with a shield icon; click it to open the workspace.
Everything below is a tab of that workspace, and the tab is part of the URL, so
a link opens exactly what you were looking at.

On the **Overview** tab, fill in the scope, objectives, assumptions and
exclusions. They are the first thing a reviewer reads, and they appear at the
top of the report.

![The Overview tab: counts, threats by status and level, the risk matrix and what is missing](../screenshots/09-threat-overview.png)

## 2. Describe the architecture

Start with the **zones**: regions of different trust, from 0 (untrusted, the
internet) to 4 (most trusted, for example the customer data). Then add the
**components** (services, databases, identity providers, external users…),
the **assets** they handle, and the **data flows** between them.

You can work in the tables or on the **Diagram**. Pick a type, press *Add
component*, drag from a connector dot to another component to draw a flow, and
drag a component into a zone to place it there. Both ways make the same saves.

![The data-flow diagram, with zones as dashed regions and diamonds where a flow crosses a trust boundary](../screenshots/10-threat-diagram.png)

The diamond on a flow marks a **trust-boundary crossing**: the two ends are in
zones with different trust levels. Crossings are where most threats live, so
the workspace asks the questions you would ask: is the flow authenticated, is it
encrypted, does it carry sensitive data towards a less trusted zone? Anything
you have not answered reads *Not documented* rather than defaulting to "fine",
and shows up as a warning.

![The Components tab, with owner, zone, exposure and the missing-information warnings](../screenshots/11-threat-components.png)

Tips:

- Give every component an **owner** and a **zone**. A component without either is the first thing the completeness findings list.
- Mark **internet-facing** components. Several suggestion rules start from that flag.
- Link an architecture component to the **Feature** that implements it. The mapping is what lets a threat lead you to the model-based tests of that feature.
- Positions, shapes and colours on the diagram are saved with the threat model and change nothing about the analysis.

## 3. Find threats

Open **Threats**. You have two ways to get threats in:

- **Add threat** records one yourself, classified by STRIDE (spoofing, tampering, repudiation, information disclosure, denial of service, elevation of privilege), with the components or flows it targets.
- **Suggest threats** applies a fixed set of STRIDE rules to your architecture. Each match becomes a **candidate**, with the rule, the facts that matched, what information is missing, and the controls it recommends.

A candidate is not a finding. Its explanation lists what would **confirm or
dismiss** it — typically "document the authentication method" — because a
control not being documented is a question to check, not proof that it is
missing. Confirm the ones that apply, or dismiss them with a reason. A rule
suggests a threat for a given target once; dismissing it does not bring it back.

![The Threats tab: manual threats with ratings and flags, followed by rule-suggested candidates](../screenshots/12-threat-threats.png)

A threat moves through `candidate → open → mitigated | accepted | dismissed`.
Flags tell you what needs attention: *No control*, *Controls planned only*,
*Failing test*, *Overdue*, and, once ratings are set, *Residual risk lower than
current without a verified control*.

## 4. Rate the risk

Each threat can carry up to three ratings, each a likelihood (1–5) times an
impact (1–5):

| Rating | Meaning |
| --- | --- |
| Inherent | Before any control |
| Current | With the controls that exist today |
| Residual | Expected once planned measures are in place and verified |

A missing rating is *Not assessed*, never a silent default. The **Risks** tab
shows the 5 × 5 matrix (switch between current, inherent and residual) and a
register with treatment, owner and due date. "Change levels" edits the
thresholds that map scores to Low, Moderate, High and Critical.

![The Risks tab: the matrix and the register with inherent, current and residual levels](../screenshots/13-threat-risks.png)

Choose a **treatment** for every threat: mitigate, avoid, transfer or accept.
Accepting a risk is a decision with an owner, so it asks for a rationale, who
accepts it, and an **expiry date**. When that date passes, the threat reads as
open again and is flagged, but the earlier record is kept; re-accepting with a
new date clears it.

## 5. Plan and verify controls

**Controls** are preventive, detective, corrective or compensating, and move
from *planned* through *implemented* to *verified*. *Verified* needs
**evidence**: a reference to a document, test report, scan, configuration or
review (a link and/or a note, who reviewed it, and when). Evidence is a
reference, not an uploaded file. A review-by date flags a verification that has
gone stale.

![The Controls tab: type, status, evidence count, and linked threats and security tests](../screenshots/14-threat-controls.png)

From a candidate's editor, **Add as control** creates a planned control linked
to that threat from the rule's recommendations.

## 6. Test it

**Security Tests** are a list of their own: they belong to the threat model and
do not appear on the Test Cases page, in coverage or in traceability. Write
them by hand, or use **Generate tests from threats** to draft them from
templates, one per linked control. Generated tests start as drafts; mark them
ready, run them, and **Record result** (passed, failed or not run). A failed
latest result flags the threat. If a threat is dismissed or a control is
unlinked, the tests generated for it are marked stale; they are never changed
or deleted for you.

![The Security Tests tab, with generated tests, their threats and controls and the latest result](../screenshots/15-threat-security-tests.png)

CSV and Gherkin export cover exactly the tests the table shows.

## 7. Take a baseline and report

A **baseline** is an immutable snapshot of the whole threat model at a point in
time: scope, architecture, ratings, treatments, controls, evidence and tests.
Take one before a release or a review. *Open* shows it as a read-only report,
and *Compare* lists what changed since: threats added, removed or changed,
controls whose status moved, and the number of threats per level on both sides.

![The Baselines tab](../screenshots/16-threat-baselines.png)

The **Report** tab renders the live model, or any baseline, as a threat and
risk assessment. *Print* uses a print stylesheet without the application chrome;
*Export Markdown* gives you the text.

![The Report tab](../screenshots/17-threat-report.png)

## Using the AI assistant

Each threat model has an **AI** launcher in the bottom-right corner of the
workspace. It is **off by default**: turning it on shows which provider and
which endpoint host will receive your data, and *Preview context* shows exactly
what would be sent before anything is.

Ask it what could go wrong with part of the architecture, to sketch zones,
components and flows from a description, to propose ratings or controls for a
threat, to draft security tests, or to review the model. Each answer arrives as
a **card** with its content, a rationale and the model that produced it. A
review is advisory text and has nothing to accept.

![The threat model's AI assistant over the Threats tab](../screenshots/18-threat-ai-assistant.png)

Things to know:

- **Nothing is saved until you accept it.** Accepting writes through the API and then updates the workspace; rejecting leaves no trace. Proposals are never counted in summaries, matrices or exports.
- Ratings from a proposal are applied **only on accept**, and the card shows the old and the proposed value.
- Accepted entities carry an **AI** badge and the model id as their source. AI-created threats start as open (accepting is your confirmation); AI-created tests start as drafts.
- The assistant cannot delete anything, accept or dismiss a threat, set a treatment or an acceptance, add evidence, change thresholds or mark a control verified. An answer that tries is rejected as invalid. Text from your model is sent as delimited data, and the assistant has no tools to call.
- The provider is configured in the environment; see [AI integration](../specification/07-ai-integration.md).

## Working with the rest of TestModeller

- A component mapped to a **Feature** links the threat model to the model-based tests of that feature.
- The project tree, search and tenant rules are the same as everywhere else; a threat model is deleted with its project.
- The threat-model screens are part of the documented UI, so they are regenerated with the other [screenshots](../screenshots/README.md).

---
title: 0012. Residual risk is entered by the analyst, and baselines are immutable snapshots
type: adr
status: accepted
date: 2026-10-17
tags: [domain, threat-modeling, governance]
related: [../specification/09-threat-modeling.md, 0011-threat-model-entities-and-security-tests.md]
---

# 0012. Residual risk is entered by the analyst, and baselines are immutable snapshots

## Context

The threat model rates inherent and current risk. A defensible assessment also states the residual risk and can show what was assessed at a given release. Residual risk could be derived from the status or effectiveness of linked controls; a baseline could be kept as row history or as a snapshot.

## Decision

- **Residual risk is a third analyst-entered rating**, like the current rating. It is never derived. Advisory flags (`residual-above-current`, `unsupported-residual`) surface claims that evidence does not support. Verified status requires evidence, which gives the flags something to check.
- **Baselines are immutable JSON snapshots** of the threat model, stored in one row each. Scores, levels and effective status are recomputed when a baseline is read; flags and warnings keep the values they had when it was taken. Per-row history or temporal tables are not used.
- **Evidence is a reference** (link or note), not a stored file.
- **Acceptance expiry is evaluated on read.** The stored status stays `accepted`; the API reports `effectiveStatus` and a flag. No background job changes data.

## Consequences

- No fixed percentage or hidden formula can make a risk look smaller than the analyst said.
- Deriving residual risk from control effectiveness remains possible later without changing stored data.
- Baselines cost a few hundred KB each at most; at most 50 per threat model.
- A baseline read later may show different scores, levels or effective status than when it was taken (thresholds are the baseline's own; the clock is today's); the snapshot records its `schemaVersion`.
- Anything that filters by threat status must use the effective status.

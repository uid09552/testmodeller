---
title: 0019. AI history is deleted after a retention period
type: adr
status: accepted
date: 2026-11-28
tags: [ai, retention, data-lifecycle]
related: [0013-threat-model-ai-assistance.md, ../specification/07-ai-integration.md]
---

# 0019. AI history is deleted after a retention period

## Context
Jobs and proposals were never deleted, so the tables grow with every request
and keep prompts' results (and failure texts that can quote user content)
forever.

## Decision
- A background task deletes, once an hour, finished jobs older than
  `TM_AI_JOB_RETENTION_DAYS` (7) and decided proposals older than
  `TM_AI_PROPOSAL_RETENTION_DAYS` (90). `0` disables each.
- Pending proposals and queued or running jobs are never deleted: unreviewed AI
  output is not dropped behind the user's back (AGENTS.md rule 8).
- `decided_at` is set by a trigger when a proposal leaves `pending`, so no
  writer can forget it; existing decided rows are backfilled from `created_at`.
- Jobs are purged first so a job never lists proposals that are gone.
- The runner of feature and threat model jobs is one code path.

## Consequences
- Decisions older than the period lose their rationale text; accepted entities
  keep `origin` and `source`.
- Paging of the threat model lists is a separate contract change and not part
  of this decision.

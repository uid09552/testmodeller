---
title: 0020. Playwright test skeletons as an export format
type: adr
status: accepted
date: 2026-11-28
tags: [export, results, playwright]
related: [../specification/03-requirements.md, ../specification/05-api.md]
---

# 0020. Playwright test skeletons as an export format

## Context
Results come back by `@tm-<id>` tag (FR-051), and Cucumber users get the tag
from the Gherkin export. Playwright users had no export that closes the loop.

## Decision
- `GET /projects/{id}/export?format=playwright` returns one `.spec.ts`
  (`text/typescript`): a `test.describe` per feature, a `test` per test case
  titled `<name> @tm-<id>`, and one `test.step` per step with the expected text
  as a constant to implement against. Scope parameters work as for other formats.
- Everything user-written is emitted as a JSON string literal, never as code or
  a comment, so a name or step cannot inject code into the generated file.
- No zip: one file per request keeps the endpoint a plain download. Larger
  projects export per feature or component.
- The output is a skeleton. It does not generate selectors or assertions.

## Consequences
- A Playwright JUnit report (the title carries the tag) imports without extra
  configuration.
- The `tm_export` MCP tool accepts the new format. The Test Cases page has no
  button for it yet.

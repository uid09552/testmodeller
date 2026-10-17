Task: review the threat model in the context and report up to {{count}} gaps or inconsistencies the analyst
should look at: missing owners, flows crossing trust boundaries without authentication or encryption,
critical assets without controls, threats without ratings, controls without tests, inconsistent assumptions.
This answer is advisory text only. Nothing in it is applied.

Return:
{"proposals": [{"rationale": "what the review covered", "payload": {
  "findings": [{"text": "...", "entityIds": ["<id from the context>"]}]
}}]}
Return exactly one proposal.

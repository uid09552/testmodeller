Task: propose up to {{count}} security tests for the threats (and controls) named in the instruction, or for
threats in the context that have no test. A test is a manual procedure a tester can run.

Return:
{"proposals": [{"rationale": "what the test proves", "payload": {
  "title": "...", "objective": "...",
  "type": "negative-authorization|authentication|input-validation|transport-security|configuration-review|resilience|logging-audit|other",
  "preconditions": ["..."], "steps": [{"action": "...", "expected": "..."}],
  "threatIds": ["<threat id from the context>"], "controlIds": ["<control id from the context>"]
}}]}
Every step needs an action. At least one threat id is required.

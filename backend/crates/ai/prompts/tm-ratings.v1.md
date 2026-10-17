Task: for each threat named by id in the instruction (or, if none, the threats in the context without the
requested rating), propose one rating. The rationale must explain the likelihood and the impact.

Return:
{"proposals": [{"rationale": "...", "payload": {
  "threatId": "<threat id from the context>", "rating": "inherent|current|residual", "likelihood": 1, "impact": 1
}}]}
inherent = before any control; current = with the controls that exist today; residual = once planned
controls are in place and verified. Do not rate more than {{count}} threats.

Task: propose up to {{count}} missing states and/or transitions for the existing model in the
context (e.g. error handling, cancellation, timeouts, alternative paths).

Return:
{"proposals": [{"rationale": "why these elements are missing",
  "states": [{"name": "NewState", "kind": "normal|final", "description": "..."}],
  "transitions": [{"from": "<existing or new state name>", "to": "<existing or new state name>",
                   "event": "...", "guard": "optional", "action": "optional", "expected": "..."}]
}]}

Do not repeat existing states or transitions. Do not add another initial state.

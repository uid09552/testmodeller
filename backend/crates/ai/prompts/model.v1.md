Task: propose {{count}} state machine model(s) for the feature described in the context.

Return:
{"proposals": [{"rationale": "why this model", "model": {
  "name": "...", "description": "...",
  "variables": [{"name": "attempts", "type": "integer|boolean|string", "initial": 0}],
  "states": [{"name": "...", "kind": "initial|normal|final", "description": "..."}],
  "transitions": [{"from": "<state name>", "to": "<state name>", "event": "...",
                   "guard": "optional", "action": "optional", "expected": "observable result"}]
}}]}

Each model must have exactly one initial state, unique state names, and transitions that only
reference its own states.

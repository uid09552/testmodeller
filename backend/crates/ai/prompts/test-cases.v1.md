Task: propose {{count}} additional test cases for the feature and model in the context, focusing on
edge cases and negative scenarios not covered by the existing test cases listed there.

Return:
{"proposals": [{"rationale": "what risk this test covers", "testCase": {
  "name": "...", "description": "...", "preconditions": "...", "priority": "low|medium|high",
  "steps": [{"action": "...", "expected": "..."}]
}}]}

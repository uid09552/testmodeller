Task: propose up to {{count}} security controls for the threats named in the instruction (or, if none, for
threats in the context without a control). Prefer linking an existing control when one already fits.

Return:
{"proposals": [{"rationale": "which threats this control addresses and how", "payload": {
  "name": "...", "type": "preventive|detective|corrective|compensating",
  "category": "identity-access|data-protection|application-network|detection-response|secure-development",
  "description": "...", "acceptanceCriteria": "...", "componentIds": ["<component id>"],
  "threatIds": ["<threat id from the context>"]
}}]}
To link an existing control instead of creating one, return {"linkControlId": "<control id>", "threatIds": [...]}
with no other fields.

Task: propose up to {{count}} additional threats that the context does not list yet, grounded in the
components, flows, trust boundaries and assets in the context. Prefer the targets named in the instruction.
Each threat needs at least one existing target (a component or flow id from the context).

Return:
{"proposals": [{"rationale": "why this threat applies to this architecture", "payload": {
  "title": "...", "category": "spoofing|tampering|repudiation|information-disclosure|denial-of-service|elevation-of-privilege",
  "scenario": "...", "attackerObjective": "...", "preconditions": "...", "consequences": "...", "threatSource": "...",
  "targets": [{"kind": "component|flow", "id": "<id from the context>"}],
  "assetIds": ["<asset id from the context>"]
}}]}

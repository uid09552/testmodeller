Task: from the analyst's instruction, propose the architecture elements that are missing from the context.
Return exactly one proposal containing only NEW elements. Existing zones, components and assets are listed
in the context; do not repeat them, but you may refer to them by name.

Return:
{"proposals": [{"rationale": "why these elements", "payload": {
  "zones": [{"name": "...", "kind": "internet|partner|dmz|internal|restricted-data|management|identity|cicd|third-party|other", "trustLevel": 0, "description": "..."}],
  "assets": [{"name": "...", "category": "business-data|identity-secrets|software-code|infrastructure|security-asset|operational-asset|business-capability", "confidentiality": "low|medium|high", "integrity": "low|medium|high", "availability": "low|medium|high", "personalData": false}],
  "components": [{"name": "...", "type": "web-frontend|mobile-app|desktop-client|browser-extension|api-gateway|api|message-broker|webhook|external-connector|service|worker|scheduler|serverless-function|relational-database|object-storage|cache|search-index|file-store|backup|identity-provider|authorization-service|secrets-manager|key-management|audit-service|cloud-service|container-platform|cicd-pipeline|third-party-saas|library|human-user|external-system", "zone": "zone name", "owner": "...", "technology": "...", "criticality": "low|medium|high|critical", "internetFacing": false, "assets": ["asset name"]}],
  "flows": [{"source": "component name", "target": "component name", "relationship": "calls|sends-data|reads-writes|authenticates-through|trusts|depends-on|runs-on|publishes-subscribes|replicates-backs-up|deploys", "protocol": "...", "authentication": "none|unknown|a method", "encrypted": "yes|no|unknown", "assets": ["asset name"], "entryPoint": false}]
}}]}
Trust levels: 0 untrusted (internet) to 4 most trusted. Names must be unique and must not already exist.
Only state authentication and encryption when the instruction says so; otherwise use "unknown".

# TestModeller

Model-based testing (MBT) tool. Users model scenarios, group them into features and components, generate test cases from the models, and get AI-assisted proposals.

- Backend: Rust (`backend/`, Cargo workspace)
- Frontend: Angular (`frontend/`)
- Docs and specification: [docs/](docs/README.md)
- Agent guidance: [AGENTS.md](AGENTS.md)

## Layout

```
.
├── AGENTS.md                  # Entry point for AI agents
├── .github/
│   ├── copilot-instructions.md
│   ├── agents/                # Custom agents
│   ├── instructions/          # File-scoped instructions
│   ├── prompts/               # Reusable prompts
│   └── skills/                # Development skills
├── docs/
│   ├── specification/         # What to build
│   ├── architecture/          # How it is built
│   ├── guides/                # Coding and workflow guides
│   └── adr/                   # Architecture decision records
├── backend/                   # Rust workspace
│   └── crates/{domain,generation,ai,storage,api}
├── frontend/                  # Angular app (to be scaffolded)
└── scripts/
```

## Status

Specification phase. No implementation yet.

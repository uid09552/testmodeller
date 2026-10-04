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

## Running with Docker

```bash
cp .env.example .env     # set POSTGRES_PASSWORD
make up                  # builds and starts frontend, backend and postgres
```

The app is on <http://localhost:8088>. The backend API is also published on
<http://127.0.0.1:8081/api/v1> for direct access.

| Target | What it does |
| --- | --- |
| `make up` | Build and start the stack in the background |
| `make ps` | Container status and health |
| `make logs` | Follow logs from all containers |
| `make down` | Stop the stack, keep the database volume |
| `make down-volumes` | Stop the stack and delete the database volume |

For local development without containers, `make dev` runs the backend against a
throwaway database (needs Docker) and `make dev-fe` runs the Angular dev server.

See [ADR 0002](docs/adr/0002-container-deployment.md) for how the images are
built and why nginx resolves the backend per request.

## Status

Specification phase. No implementation yet.

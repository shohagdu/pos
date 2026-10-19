# SaaS POS

Multi-tenant point-of-sale and business management platform for retail
businesses in Bangladesh.

| Path | Contents |
|---|---|
| [`Final_docs/`](Final_docs/README.md) | Requirement analysis, development roadmap, database design (MySQL) |
| [`backend/`](backend) | Rust workspace: `pos-api` (Actix Web), `pos-worker`, shared crates, MySQL migrations |
| [`frontend/`](frontend) | React 19 + TypeScript + Vite + Tailwind SPA |
| [`infra/`](infra) | Docker Compose for local dependencies, MySQL config, edge Nginx |
| [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) | Local setup, commands, conventions |

Quick start (Docker required for dependencies):

```bash
make deps-up                          # MySQL 8.4, Redis, MinIO, Mailpit
cp backend/.env.example backend/.env
make api                              # applies migrations, serves :8080
make web                              # http://localhost:5173
```

**Status:** Phase 1 (Foundation) in progress. See the roadmap, `Final_docs/02_Development_Roadmap.md` §Phase 1.
# pos

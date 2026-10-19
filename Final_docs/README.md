# SaaS POS — Final Documentation

These documents were built from the initial documents in `../Docs/` (feature list, SRS, development roadmap).

**Stack:** React + TypeScript + Tailwind CSS · Rust + Actix Web + SQLx · **MySQL 8.4 LTS** · Redis · Docker

| # | Document | What it contains |
|---|---|---|
| 01 | [Requirement Analysis](01_Requirement_Analysis.md) | Conflicts and gaps found in the initial documents, **decision register (D-01…D-20) with recommended answers to every open question**, personas, MVP scope, user stories and acceptance criteria per module, pricing/tax/WAC calculation rules with worked examples, state machines, permission matrix, plan entitlements, quantified NFRs, API conventions, screens, reports catalogue, acceptance tests, traceability |
| 02 | [Development Roadmap](02_Development_Roadmap.md) | Architecture (Rust workspace and React structure), engineering standards, phases 0–8 with epics, migrations and exit criteria, **sprint-by-sprint MVP plan (32 weeks)**, dependency graph, release gates, testing strategy, CI/CD and environments, team/RACI, risks, KPIs, first 10 days |
| 03 | [Database Design (MySQL)](03_Database_Design.md) | Tenant isolation without RLS (tenant-scoped data layer, `PRIMARY KEY (tenant_id, id)` + composite FKs, guard triggers, CI SQL lint), MySQL server settings, ERDs, **full DDL for 91 tables** (MVP + Release 2), posting workflows (sale, return, purchase, collection), indexing/partitioning, migration plan, seed data, reconciliation and reporting queries, backup/retention, capacity estimates, PostgreSQL → MySQL change log |

**Status:** final draft for approval. Every decision marked *Recommended default* in doc 01 §3 needs product owner sign-off before Sprint 1.

**Verification:** the DDL in doc 03 §9 was run on MySQL 8.0.46, and the design targets 8.4 LTS. The following checks passed:
- All 91 tables create with no errors, and the CI schema guards pass.
- Guard triggers block cross-tenant and context-less writes (insert, update and delete), and platform bypass works.
- Composite foreign keys reject cross-tenant links.
- Append-only triggers protect the ledgers.
- The partial-unique patterns work: default branch, default variant, walk-in customer, and SKU reuse after soft delete.
- The balanced-journal CHECK and the period-lock trigger work.
- Bengali FULLTEXT ngram search, the `JSON_TABLE` batch update, the `LAST_INSERT_ID` sequence and partition reorganisation work.

#!/usr/bin/env python3
"""CI lint: every SQL statement that touches a tenant table must filter or
bind tenant_id (DB design §3.1 layer L3).

MySQL has no row-level security, so a query that forgets its tenant filter
would read other tenants' data. This lint scans:
  * SQL string literals in crates/**/*.rs
  * the SQLx offline cache (.sqlx/*.json), once compile-time macros are used

Tenant tables are discovered from migrations (CREATE TABLE ... tenant_id).
It is a heuristic: it requires the token `tenant_id` in any statement that
mentions a tenant table. Exceptions are declared inline in the SQL with
    -- tenant-lint: allow <reason>
or listed per file in ALLOWED_FILES below (platform-wide code paths).
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MIGRATIONS = ROOT / "migrations"

# Files whose queries are platform-wide by design. Each entry needs a reason.
ALLOWED_FILES = {
    "crates/db/src/guards.rs": "information_schema checks, no tenant data",
    "crates/worker/src/partitions.rs": "partition DDL on whole tables",
    "crates/worker/src/cleanup.rs": "retention purge across all tenants (platform tx)",
}

SQL_START = re.compile(r"^\s*(SELECT|INSERT|UPDATE|DELETE|REPLACE|WITH)\b", re.I)
# Rust string literals: "..." and r"..." / r#"..."#
RUST_STRING = re.compile(r'r(#*)"(.*?)"\1|"((?:[^"\\]|\\.)*)"', re.S)


def tenant_tables() -> set[str]:
    tables = set()
    for path in sorted(MIGRATIONS.glob("*.sql")):
        sql = path.read_text(encoding="utf-8")
        for name, body in re.findall(r"CREATE TABLE\s+(?:IF NOT EXISTS\s+)?`?(\w+)`?\s*\((.*?)\n\)", sql, re.S | re.I):
            if re.search(r"^\s*tenant_id\s", body, re.M):
                tables.add(name)
    return tables


def statements_from_rust(text: str) -> list[tuple[int, str]]:
    out = []
    for m in RUST_STRING.finditer(text):
        s = m.group(2) if m.group(2) is not None else m.group(3)
        # join Rust line continuations ("...\<newline>   ...")
        s = re.sub(r"\\\n\s*", "", s)
        if SQL_START.match(s):
            out.append((text.count("\n", 0, m.start()) + 1, s))
    return out


def statements_from_sqlx_cache(path: Path) -> list[tuple[int, str]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    return [(0, data.get("query", ""))]


def violations(stmt: str, tables: set[str]) -> list[str]:
    if "tenant-lint: allow" in stmt:
        return []
    referenced = {t for t in tables if re.search(rf"\b{t}\b", stmt)}
    if referenced and not re.search(r"\btenant_id\b", stmt):
        return sorted(referenced)
    return []


def main() -> int:
    tables = tenant_tables()
    problems = []

    for path in sorted((ROOT / "crates").rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        if rel in ALLOWED_FILES or "/tests/" in rel:
            continue
        for line, stmt in statements_from_rust(path.read_text(encoding="utf-8")):
            bad = violations(stmt, tables)
            if bad:
                problems.append(f"{rel}:{line}: query on {', '.join(bad)} without tenant_id")

    for path in sorted((ROOT / ".sqlx").glob("*.json")) if (ROOT / ".sqlx").exists() else []:
        for _, stmt in statements_from_sqlx_cache(path):
            bad = violations(stmt, tables)
            if bad:
                problems.append(f"{path.relative_to(ROOT)}: query on {', '.join(bad)} without tenant_id")

    if problems:
        print("Tenant SQL lint failed (DB design §3.1 L3):", *problems, sep="\n  ", file=sys.stderr)
        return 1
    print(f"tenant SQL lint passed ({len(tables)} tenant tables)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

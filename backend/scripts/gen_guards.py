#!/usr/bin/env python3
"""Generate tenant-guard and append-only triggers for a migration file.

MySQL has no row-level security, so writes are protected by BEFORE triggers
that compare each row's tenant_id with the session variable @app_tenant_id
(DB design §3.1 layer L5, §9.18).

Usage:
    scripts/gen_guards.py migrations/0004_tenancy.sql [more files...]
    scripts/gen_guards.py --check migrations/*.sql     # CI: fail if out of date

The script rewrites the block between the GENERATED markers at the end of each
file. Run it only on migrations that have not been applied anywhere yet:
editing an applied migration changes its checksum and sqlx refuses to start.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

BEGIN = "-- >>> GENERATED GUARDS (scripts/gen_guards.py) - do not edit by hand"
END = "-- <<< GENERATED GUARDS"

# Tables that carry tenant_id but are written outside a tenant context.
NO_TENANT_GUARD = {"user_sessions", "auth_events", "background_jobs", "audit_logs"}

# Ledgers and logs: never updated, deleted only by the maintenance purge.
APPEND_ONLY = {
    "stock_movements",
    "party_ledger_entries",
    "account_transactions",
    "audit_logs",
    "subscription_events",
    "auth_events",
    "journal_lines",
}

CREATE_TABLE = re.compile(r"CREATE TABLE\s+(?:IF NOT EXISTS\s+)?`?(\w+)`?\s*\((.*?)\n\)", re.S | re.I)


def tables_in(sql: str) -> list[tuple[str, bool]]:
    """Return (table, has_tenant_id) for every CREATE TABLE in the file."""
    out = []
    for name, body in CREATE_TABLE.findall(sql):
        has_tenant = re.search(r"^\s*tenant_id\s", body, re.M) is not None
        out.append((name, has_tenant))
    return out


def tenant_guards(table: str, append_only: bool) -> str:
    # NULL-safe <=> everywhere: with '=' an unset variable makes the condition
    # NULL and the IF silently never fires (found in schema testing).
    parts = [
        f"""CREATE TRIGGER trg_{table}_tg_ins BEFORE INSERT ON {table} FOR EACH ROW
BEGIN
  IF NOT (NEW.tenant_id <=> @app_tenant_id OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;"""
    ]
    if not append_only:
        parts.append(
            f"""CREATE TRIGGER trg_{table}_tg_upd BEFORE UPDATE ON {table} FOR EACH ROW
BEGIN
  IF NOT ((OLD.tenant_id <=> @app_tenant_id AND NEW.tenant_id <=> @app_tenant_id) OR @app_platform <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;"""
        )
        parts.append(
            f"""CREATE TRIGGER trg_{table}_tg_del BEFORE DELETE ON {table} FOR EACH ROW
BEGIN
  IF NOT (OLD.tenant_id <=> @app_tenant_id OR @app_platform <=> 1 OR @app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'TENANT_GUARD';
  END IF;
END;"""
        )
    return "\n".join(parts)


def append_only_guards(table: str) -> str:
    return f"""CREATE TRIGGER trg_{table}_ao_upd BEFORE UPDATE ON {table} FOR EACH ROW
BEGIN
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
END;
CREATE TRIGGER trg_{table}_ao_del BEFORE DELETE ON {table} FOR EACH ROW
BEGIN
  IF NOT (@app_maintenance <=> 1) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'APPEND_ONLY';
  END IF;
END;"""


def generated_block(sql: str) -> str | None:
    sections = []
    for table, has_tenant in tables_in(sql):
        chunk = []
        if has_tenant and table not in NO_TENANT_GUARD:
            chunk.append(tenant_guards(table, table in APPEND_ONLY))
        if table in APPEND_ONLY:
            chunk.append(append_only_guards(table))
        if chunk:
            sections.append(f"-- {table}\n" + "\n".join(chunk))
    if not sections:
        return None
    return f"{BEGIN}\n\n" + "\n\n".join(sections) + f"\n\n{END}\n"


def render(sql: str) -> str:
    base = sql.split(BEGIN)[0].rstrip() + "\n"
    block = generated_block(base)
    return base if block is None else f"{base}\n{block}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("files", nargs="+", type=Path)
    parser.add_argument("--check", action="store_true", help="exit 1 if any file is out of date")
    args = parser.parse_args()

    stale = []
    for path in args.files:
        current = path.read_text(encoding="utf-8")
        wanted = render(current)
        if wanted != current:
            if args.check:
                stale.append(path)
            else:
                path.write_text(wanted, encoding="utf-8")
                print(f"updated {path}")
    if stale:
        print("guard triggers out of date, run scripts/gen_guards.py on:", *stale, sep="\n  ", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

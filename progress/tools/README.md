# Audit tools

- `audit.mjs`: headless-Chrome audit. Run from this folder with the dev server on :5173 (API on :8080):
  `node audit.mjs out.json <screenshot-dir or -> [route regex]`; env `SIZES=360x640,1366x768` limits sizes.
  Logs in as 01700000001 / Test@12345, sets Bengali, picks real IDs from the API, measures each route.
  Log line per size: `X` page scrolls sideways, `T` table scrolls in its frame, `C` amounts cut off, `O` element past the edge.
- `gen2.py`: builds `../audit.md` and `../pages/*.md` from `before.json` + `after.json` (edit the `P` path / `SPECIFIC` notes as needed; it reuses helpers from `gen.py`).
- `before.json` / `after.json`: raw measurements from session 1.

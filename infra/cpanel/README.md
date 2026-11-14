# Deploying to cPanel shared hosting (no SSH)

Target: `https://jsaebike.com`, LiteSpeed + cPanel "Setup Node.js App".

`app.js` is a dependency-free Node launcher. It starts `bin/pos-api` and
`bin/pos-worker` (static musl builds, so the host's glibc does not matter),
serves the SPA from `public/`, and forwards `/api`, `/healthz` and `/readyz`
to pos-api on `127.0.0.1:API_INTERNAL_PORT`. Logs go to `logs/`.

## Host requirements (check first)

- **MySQL 8.0+ or MariaDB 10.5+.** Every migration must run on both: CI migrates a fresh
  MariaDB 10.5 database (job `mariadb` in `.github/workflows/ci.yml`) on every push and pull
  request. Before building a release without CI, run the same check by hand:
  `docker run -d --name mdb -p 3307:3306 -e MARIADB_ROOT_PASSWORD=root -e MARIADB_DATABASE=pos_ci mariadb:10.5 --log-bin-trust-function-creators=1`,
  then `DATABASE_URL=mysql://root:root@127.0.0.1:3307/pos_ci APP_ENV=test JWT_SECRET=x cargo run -p pos-api -- migrate`
  (twice) and `-- check-schema`. Never edit a migration production has applied: if one has to
  change, add its old checksum to `RELEASED_CHECKSUMS` in `backend/crates/db/src/migrations.rs`.
  cPanel > General Information (right column) or phpMyAdmin's home page shows
  the server version. Start with an empty database: the app creates the tables
  itself, so do not import a dump from another server.
- **Triggers**: the migrations create tenant-guard triggers. If the log shows
  `You do not have the SUPER privilege and binary logging is enabled`, ask the
  host to set `log_bin_trust_function_creators=1` or to grant TRIGGER for your
  database user.
- The host must allow the Node app to start child processes and keep them
  running (CloudLinux limits: entry processes, memory ≥ 512 MB recommended).

## 1. Build the package (on a dev machine)

```bash
infra/cpanel/build.sh        # -> deploy/pos-cpanel.zip (Bangla guide: deploy/README-bn.md)
```

## 2. Database

cPanel > **MySQL Databases**:

1. Create database `pos` (it becomes `CPUSER_pos`).
2. Create user `posapp` (`CPUSER_posapp`) with a strong password.
3. Add the user to the database with **ALL PRIVILEGES**.

Tables are created automatically on first start (`RUN_MIGRATIONS_ON_START=true`).

## 3. Upload

cPanel > **File Manager**:

1. Go to your home directory (not `public_html`).
2. Upload `pos-cpanel.zip` there and **Extract** it. The zip holds a
   `pos-app/` folder, so this creates `~/pos-app` (do not create it first).
3. Rename `env.example` to `.env` (enable *Show Hidden Files* in Settings),
   then **Edit** it: `DATABASE_URL`, `JWT_SECRET` (`openssl rand -base64 48`).
4. Select `bin/pos-api` and `bin/pos-worker` > **Permissions** > `755`.
5. Move any existing files out of `public_html` (the domain will be served by
   the Node app; an `index.html`/`index.php` there can take precedence).

## 4. Node.js app

cPanel > **Setup Node.js App** > **Create Application**:

| Field | Value |
|---|---|
| Node.js version | 18 or newer |
| Application mode | Production |
| Application root | `pos-app` |
| Application URL | `jsaebike.com` (leave the path empty) |
| Application startup file | `app.js` |

**Create**, then **Start App** (no *Run NPM Install* needed).

## 5. Verify

- `https://jsaebike.com/healthz` → `{"status":"ok"}`
- `https://jsaebike.com/readyz` → `{"status":"ready"}` (database reachable)
- `https://jsaebike.com/` → login page
- Problems: File Manager > `pos-app/logs/` → `launcher.log`, `pos-api.log`,
  `pos-worker.log`.

## Updating

Build again, upload the zip to the home directory and extract it over
`pos-app` (the zip has no `.env`, and existing `storage/` files are kept),
re-apply `755` on the binaries, then **Restart** in Setup Node.js App.

## Known limits

- `APP_ENV=staging`: pos-api refuses `production` until an SMS gateway is
  integrated. Sign-up does not ask for an SMS code (`AUTH_PHONE_VERIFICATION`
  is off). Password-reset codes and invitation links are only written to
  `logs/pos-api.log`, so someone with File Manager access has to read them
  from the log.
- When the host idles the Node app out, the API stops with it; the first
  request after that waits a few seconds (`503 API_STARTING`) while it boots.
- Uploaded product images live in `pos-app/storage/`; include it in backups
  along with the database (cPanel > Backup).

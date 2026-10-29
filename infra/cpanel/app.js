// cPanel "Setup Node.js App" launcher for shared hosting without SSH.
//
// Starts bin/pos-api and bin/pos-worker as child processes, serves the built
// SPA from public/, and forwards /api, /healthz and /readyz to pos-api on
// 127.0.0.1. Uses only Node built-ins, so "Run NPM Install" is not needed.
//
// Layout of the application root (next to this file):
//   app.js  .env  bin/pos-api  bin/pos-worker  public/  storage/  logs/
'use strict'

const fs = require('fs')
const http = require('http')
const path = require('path')
const { spawn } = require('child_process')

const ROOT = __dirname

// Without SSH, a crash before logs/ exists is otherwise invisible (LiteSpeed
// only shows "503 Service Unavailable").
process.on('uncaughtException', (err) => {
  try {
    fs.appendFileSync(path.join(ROOT, 'startup-error.log'), `${new Date().toISOString()} ${err.stack || err}\n`)
  } catch (e) {}
  try {
    for (const child of children.values()) child.kill('SIGTERM')
  } catch (e) {}
  process.exit(1)
})

const PUBLIC_DIR = path.join(ROOT, 'public')
const LOG_DIR = path.join(ROOT, 'logs')
const MAX_LOG_BYTES = 10 * 1024 * 1024

const env = readDotenv(path.join(ROOT, '.env'))
// Internal port for pos-api. Shared hosts put many accounts on one machine, so
// the launcher moves to a random port if this one turns out to be taken.
let apiPort = Number(env.API_INTERNAL_PORT || 18080)
// Set when cPanel's database user only works over the unix socket.
let databaseUrl = env.DATABASE_URL
let apiUp = false

fs.mkdirSync(LOG_DIR, { recursive: true })
fs.mkdirSync(path.join(ROOT, env.STORAGE_DIR || 'storage'), { recursive: true })

const children = new Map()

function readDotenv(file) {
  const out = {}
  if (!fs.existsSync(file)) return out
  for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
    const m = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$/)
    if (!m) continue
    out[m[1]] = m[2].replace(/^(['"])(.*)\1$/, '$2')
  }
  return out
}

function logStream(name) {
  const file = path.join(LOG_DIR, `${name}.log`)
  try {
    if (fs.statSync(file).size > MAX_LOG_BYTES) fs.renameSync(file, `${file}.1`)
  } catch {}
  return fs.createWriteStream(file, { flags: 'a' })
}

function launcherLog(msg) {
  fs.appendFileSync(path.join(LOG_DIR, 'launcher.log'), `${new Date().toISOString()} ${msg}\n`)
}

// Stops a pos-api/pos-worker left behind by a previous launcher that was
// killed without cleaning up (it would still hold the port).
function stopStale(name) {
  const pidFile = path.join(LOG_DIR, `${name}.pid`)
  try {
    const pid = Number(fs.readFileSync(pidFile, 'utf8'))
    const cmd = fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8')
    if (cmd.includes(path.join(ROOT, 'bin', name))) {
      process.kill(pid, 'SIGTERM')
      launcherLog(`stopped stale ${name} pid ${pid}`)
    }
  } catch {}
}

function start(name, args) {
  const bin = path.join(ROOT, 'bin', name)
  try {
    fs.chmodSync(bin, 0o755)
  } catch {}
  const out = logStream(name)
  const child = spawn(bin, args, {
    cwd: ROOT,
    // Override .env so pos-api only listens on loopback behind this proxy.
    env: {
      ...process.env,
      HTTP_HOST: '127.0.0.1',
      HTTP_PORT: String(apiPort),
      DATABASE_URL: databaseUrl,
      LOG_FORMAT: 'json',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  // Both streams share one file: end it once, after the child has fully closed.
  child.stdout.pipe(out, { end: false })
  child.stderr.pipe(out, { end: false })
  child.on('close', () => out.end())
  // EAGAIN here means the host's process limit was hit. Without a handler the
  // 'error' event would take the whole launcher down.
  child.on('error', (err) => {
    launcherLog(`${name} could not start: ${err.message}`)
    if (child.pid !== undefined) return
    children.delete(name)
    out.end()
    if (!shuttingDown) setTimeout(() => start(name, args), 15000)
  })
  fs.writeFileSync(path.join(LOG_DIR, `${name}.pid`), String(child.pid))
  children.set(name, child)
  const startedAt = Date.now()
  launcherLog(`started ${name} pid ${child.pid}`)
  child.on('exit', (code, signal) => {
    children.delete(name)
    launcherLog(`${name} exited code=${code} signal=${signal}`)
    if (shuttingDown) return
    if (name === 'pos-api') autoFix(tail(path.join(LOG_DIR, 'pos-api.log'), 4096))
    // Back off when it crashes straight away (bad .env, database down).
    const delay = Date.now() - startedAt < 10000 ? 15000 : 2000
    setTimeout(() => start(name, args), delay)
  })
}

function tail(file, bytes) {
  try {
    const size = fs.statSync(file).size
    const fd = fs.openSync(file, 'r')
    const buf = Buffer.alloc(Math.min(bytes, size))
    fs.readSync(fd, buf, 0, buf.length, size - buf.length)
    fs.closeSync(fd)
    return buf.toString('utf8')
  } catch (e) {
    return ''
  }
}

const SOCKETS = ['/var/lib/mysql/mysql.sock', '/var/run/mysqld/mysqld.sock', '/tmp/mysql.sock']

// Repairs the two start-up failures that need no human decision.
function autoFix(log) {
  if (/Address already in use|os error 98/.test(log)) {
    apiPort = 20000 + Math.floor(Math.random() * 40000)
    launcherLog(`port taken; pos-api moves to 127.0.0.1:${apiPort}`)
  } else if (/Access denied for user '[^']*'@'(127\.0\.0\.1|::1)'/.test(log) && !/[?&]socket=/.test(databaseUrl)) {
    const socket = SOCKETS.find((s) => fs.existsSync(s))
    if (socket) {
      databaseUrl += `${databaseUrl.includes('?') ? '&' : '?'}socket=${socket}`
      launcherLog(`database refused TCP login; retrying over ${socket}`)
      const worker = children.get('pos-worker')
      if (worker) worker.kill('SIGTERM')
    }
  }
}

// Polled so /pos-status can tell "starting" from "crashing"; /readyz also
// checks the database, which pos-api only connects to lazily.
function checkHealth() {
  const req = http.get({ host: '127.0.0.1', port: apiPort, path: '/readyz', timeout: 3000 }, (res) => {
    apiUp = res.statusCode === 200
    res.resume()
  })
  req.on('timeout', () => req.destroy())
  req.on('error', () => {
    apiUp = false
  })
}

let shuttingDown = false
function shutdown() {
  shuttingDown = true
  for (const child of children.values()) child.kill('SIGTERM')
  setTimeout(() => process.exit(0), 3000).unref()
}
process.on('SIGTERM', shutdown)
process.on('SIGINT', shutdown)

// --- reverse proxy --------------------------------------------------------

function isApiPath(url) {
  return url.startsWith('/api/') || url === '/healthz' || url === '/readyz' || /^\/(healthz|readyz)\?/.test(url)
}

function clientIp(req) {
  // LiteSpeed appends the connecting address, so the last entry is the one
  // it saw; earlier entries come from the client and can be forged.
  const xff = String(req.headers['x-forwarded-for'] || '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
  return xff[xff.length - 1] || req.socket.remoteAddress || ''
}

function proxy(req, res) {
  const headers = { ...req.headers }
  const ip = clientIp(req)
  if (ip) headers['x-forwarded-for'] = ip
  else delete headers['x-forwarded-for']
  delete headers.forwarded
  headers['x-forwarded-proto'] = 'https'
  const upstream = http.request(
    { host: '127.0.0.1', port: apiPort, method: req.method, path: req.url, headers },
    (up) => {
      res.writeHead(up.statusCode, up.headers)
      up.pipe(res)
    },
  )
  upstream.on('error', () => {
    if (!res.headersSent) {
      res.writeHead(503, { 'content-type': 'application/json', 'retry-after': '5' })
    }
    res.end(
      JSON.stringify({
        error: { code: 'API_STARTING', message: 'The server is starting, try again shortly. Details: /pos-status' },
      }),
    )
  })
  req.pipe(upstream)
}

// --- static SPA -------------------------------------------------------------

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',
  '.woff2': 'font/woff2',
  '.txt': 'text/plain; charset=utf-8',
  '.webmanifest': 'application/manifest+json',
}

function sendFile(res, file, method) {
  const ext = path.extname(file)
  const immutable = file.startsWith(path.join(PUBLIC_DIR, 'assets') + path.sep)
  res.writeHead(200, {
    'content-type': MIME[ext] || 'application/octet-stream',
    'cache-control': immutable ? 'public, max-age=31536000, immutable' : 'no-cache',
    'x-content-type-options': 'nosniff',
  })
  if (method === 'HEAD') return res.end()
  fs.createReadStream(file).pipe(res)
}

function serveStatic(req, res) {
  if (req.method !== 'GET' && req.method !== 'HEAD') {
    res.writeHead(405, { allow: 'GET, HEAD' })
    return res.end()
  }
  let pathname
  try {
    pathname = decodeURIComponent(new URL(req.url, 'http://x').pathname)
  } catch {
    res.writeHead(400)
    return res.end()
  }
  const file = path.normalize(path.join(PUBLIC_DIR, pathname))
  if (file.startsWith(PUBLIC_DIR + path.sep)) {
    try {
      if (fs.statSync(file).isFile()) return sendFile(res, file, req.method)
    } catch {}
  }
  // Missing hashed asset: a real 404, not the SPA shell.
  if (pathname.startsWith('/assets/')) {
    res.writeHead(404)
    return res.end()
  }
  sendFile(res, path.join(PUBLIC_DIR, 'index.html'), req.method)
}

// --- /pos-status ------------------------------------------------------------
//
// Start-up diagnostics for hosts without SSH. Shows recent errors only while
// the API is down; once it is up the page says so and nothing else.

const HINTS = [
  [/Access denied for user/, 'Wrong database user/password, or the user is not added to the database. cPanel > MySQL Databases > "Add User To Database" (ALL PRIVILEGES), then check DATABASE_URL in pos-app/.env.'],
  [/Unknown database/, 'The database name in DATABASE_URL does not exist. Use the full name with your cPanel prefix (e.g. jsaebike_pos).'],
  [/UUID_TO_BIN|You have an error in your SQL syntax|MariaDB/i, 'The host runs MariaDB; this app needs MySQL 8. Ask the hosting company to switch your account to MySQL 8.'],
  [/SUPER privilege|log_bin_trust_function_creators/, 'The host blocks database triggers. Ask the hosting company to set log_bin_trust_function_creators=1 for your database.'],
  [/failed to lookup address|Name does not resolve|Name or service not known/, 'The database host in DATABASE_URL is unreadable, usually because the password contains @ # $ : / ? unencoded. Use a password of letters and digits only, or encode them (@ -> %40, # -> %23, $ -> %24, : -> %3A, / -> %2F, ? -> %3F).'],
  [/Unknown system variable 'transaction_isolation'/, 'The host runs MariaDB or MySQL older than 5.7.20; this app needs MySQL 8. Check the version in cPanel > General Information (or phpMyAdmin) and ask the hosting company for MySQL 8.'],
  [/EAGAIN/, 'The host refused to start another process (CloudLinux process limit). Stop and Start the app once; if it repeats, ask the hosting company to raise the process (NPROC) limit.'],
  [/Connection refused|Can't connect|Connection timed out/i, 'Cannot reach MySQL. Keep "localhost" as the host in DATABASE_URL.'],
  [/invalid configuration|missing required environment variable/, 'A setting in pos-app/.env is missing or invalid; the line above names it.'],
  [/Address (already )?in use/, 'The internal port was taken; the launcher has already picked another one. Reload in 30 seconds.'],
  [/Permission denied|Exec format error|EACCES/, 'The host does not allow running bin/pos-api. Ask the hosting company whether executing programs from your home directory is allowed.'],
]

function redact(text) {
  // Greedy up to the last '@': an unencoded '@' in the password must not
  // leave the rest of it visible.
  let out = text.replace(/(mysql:\/\/[^:\s]+:)\S*@/g, '$1***@')
  for (const secret of [env.JWT_SECRET, env.SMTP_PASSWORD, (/:\/\/[^:]+:(\S+)@/.exec(env.DATABASE_URL || '') || [])[1]]) {
    if (secret && secret.length >= 4) out = out.split(secret).join('***')
  }
  return out
}

function errorLines(log) {
  return log
    .split('\n')
    .filter((l) => l.trim() && (!l.startsWith('{') || /"level":"(ERROR|WARN)"/.test(l)))
    .slice(-15)
}

function status(req, res) {
  res.writeHead(200, { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' })
  if (apiUp) return res.end('OK: the API is running.\n')
  const apiLog = tail(path.join(LOG_DIR, 'pos-api.log'), 16384)
  const errors = errorLines(apiLog).concat(errorLines(tail(path.join(ROOT, 'startup-error.log'), 4096)))
  const joined = errors.join('\n')
  const hints = HINTS.filter(([re]) => re.test(joined)).map(([, hint]) => `* ${hint}`)
  const db = redact(databaseUrl || '(not set)')
  res.end(
    redact(
      [
        'The API is NOT running.',
        '',
        'What to do:',
        hints.length ? hints.join('\n') : '* No known error yet. Wait 30 seconds and reload; if this stays, send this page to support.',
        '',
        `Database: ${db}`,
        `API port: ${apiPort}   API process: ${children.has('pos-api') ? 'running' : 'not running'}`,
        '',
        '--- recent errors (logs/pos-api.log) ---',
        joined || '(none)',
        '',
        '--- logs/launcher.log ---',
        tail(path.join(LOG_DIR, 'launcher.log'), 2048).split('\n').slice(-12).join('\n'),
      ].join('\n'),
    ) + '\n',
  )
}

// --- boot -------------------------------------------------------------------

// .env still has the template's placeholders: say so instead of crash-looping.
const unconfigured = !env.DATABASE_URL || /CPUSER|CHANGE_ME/.test(env.DATABASE_URL)

function notConfigured(req, res) {
  res.writeHead(503, { 'content-type': 'text/plain; charset=utf-8' })
  res.end('Not configured yet: set DATABASE_URL in pos-app/.env, then Restart the app in cPanel > Setup Node.js App.\n')
}

if (unconfigured) {
  launcherLog('DATABASE_URL in .env is not set; API not started')
} else {
  stopStale('pos-api')
  stopStale('pos-worker')
  start('pos-api', ['serve'])
  if (env.RUN_WORKER !== 'false') start('pos-worker', [])
  setInterval(checkHealth, 5000).unref()
}

http
  .createServer((req, res) => {
    if (unconfigured) return notConfigured(req, res)
    if (req.url === '/pos-status') return status(req, res)
    return isApiPath(req.url) ? proxy(req, res) : serveStatic(req, res)
  })
  .listen(process.env.PORT || 3000, () => launcherLog(`launcher listening, api on 127.0.0.1:${apiPort}`))

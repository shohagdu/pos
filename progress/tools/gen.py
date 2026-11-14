# Builds progress/audit.md, progress/pages/*.md from before.json (+ cut.json, + status.json overrides).
import json, os, re, datetime
P = '/var/www/personal/POS/progress'
before = json.load(open('before.json'))
cut = {r['route']: r for r in json.load(open('cut.json'))} if os.path.exists('cut.json') else {}
ids = json.load(open('before-ids.json'))
status = json.load(open(f'{P}/status.json')) if os.path.exists(f'{P}/status.json') else {}

def generic(route):
    g = route.split('?')[0]
    for k, v in ids.items():
        if v: g = g.replace(v, ':id')
    return g

def ptype(g):
    if g in ('/pos',): return 'POS sale screen'
    if g in ('/login','/register','/forgot-password','/reset-password','/verify','/invite','/choose-business','/account/password'): return 'Auth form'
    if g.startswith('/onboarding'): return 'Onboarding form'
    if g.startswith('/settings') or g.startswith('/admin/settings') or g == '/account/notifications': return 'Settings form'
    if re.search(r'/(new|edit|invite|import)$', g) or g in ('/products/:id','/users/roles/:id','/users/roles/new','/labels','/sales/:id/return','/purchases/:id/return'): return 'Form'
    if g in ('/', '/admin', '/reports'): return 'Dashboard'
    if g.startswith('/reports'): return 'Report'
    if ':id' in g: return 'Detail'
    return 'List / table'

def name(g):
    return g.strip('/').replace(':id','detail').replace('/','-') or 'dashboard'

def grade(route):
    r = next(x for x in before if x['route'] == route)
    c = cut.get(route, {}).get('sizes', {})
    issues = {}
    def check(ws):
        fail, warn = [], []
        for w in ws:
            m = r['sizes'][str(w)]
            width = max(m['sw'], m['vw'])
            if width > w + 1: fail.append(f'page is {width}px wide at {w}px (sideways scroll)')
            if m['inner'] and w < 1366: warn.append(f'table scrolls sideways inside its frame at {w}px ({m["inner"][0]})')
            if m['inner'] and w >= 1366: fail.append(f'table scrolls sideways at {w}px ({m["inner"][0]})')
            cm = c.get(str(w), {}).get('cutMoney')
            if cm: fail.append(f'amounts cut off with … at {w}px: {", ".join(cm[:2])}')
            if ptype(generic(route)) in ('POS sale screen',) or generic(route) in ('/products/new','/products/:id'):
                if w == 1366 and m['sh'] > m['vh'] + 1: warn.append(f'page scrolls at 1366×768 ({m["sh"]}px tall)')
        return fail, warn
    groups = {'Mobile': [360, 390], 'Tablet': [768, 1024], 'Laptop': [1366], 'Desktop': [1440, 1920]}
    out = {}
    for gname, ws in groups.items():
        f, wn = check(ws)
        out[gname] = ('Fail' if f else 'Warn' if wn else 'Pass', f + wn)
    small = {w: r['sizes'][str(w)]['small'] for w in (360, 768)}
    font = r['sizes']['390']['smallFont']
    return out, small, font, r['sizes']['1366']['path']

rows = []
for r in before:
    route = r['route']; g = generic(route)
    out, small, font, landed = grade(route)
    st = status.get(g, {}).get('status')
    if not st:
        st = 'Pending' if any(v[0] != 'Pass' for v in out.values()) else 'Passed'
    notes = status.get(g, {}).get('notes', '')
    if landed.split('?')[0] != route.split('?')[0]: notes = (notes + f' Redirects to {landed}.').strip()
    rows.append((g, name(g), ptype(g), out, st, notes, small, font, route))
    # page file
    pf = f'{P}/pages/{name(g)}.md'
    if not os.path.exists(pf) or status.get(g, {}).get('regen'):
        lines = [f'# {g}', '', f'- Page type: {ptype(g)}', f'- Audited URL: `{route}` (Bengali)', f'- Status: {st}', '', '## Audit (before)', '']
        for gname, (verdict, iss) in out.items():
            lines.append(f'- **{gname}**: {verdict}' + ('' if not iss else ''))
            for i in iss: lines.append(f'  - {i}')
        lines += ['', f'- Tap targets under 40px (incl. shared header): {small[360]} at 360px, {small[768]} at 768px', f'- Inputs under 16px font at 390px: {font}',
                  '', f'Screenshots: `progress/screenshots/before/` ({name(g)} prefix may differ for detail pages)', '', '## Changes', '', '_Not started._', '', '## Verification', '', '_Not yet._', '']
        open(pf, 'w').write('\n'.join(lines))

order = {'Fail': 0, 'Warn': 1, 'Pass': 2}
with open(f'{P}/audit.md', 'w') as f:
    f.write('# Responsive audit\n\nChecked in Bengali at 360, 390 (mobile), 768, 1024 (tablet), 1366×768 (laptop), 1440, 1920 (desktop).\n\n'
            '**Fail**: the page scrolls sideways, a table scrolls sideways on a laptop or desktop, or amounts are cut off with "…". '
            '**Warn**: a table scrolls sideways inside its own frame on a phone or tablet (fixed by switching to cards), or a work screen scrolls at 1366×768.\n\n'
            'Every page also shares the app-wide items in `shared-components.md` (header tap targets, 14px inputs, sidebar on tablets).\n\n')
    f.write('| Route | Page name | Page type | Mobile | Tablet | Laptop | Desktop | Status | Notes |\n|---|---|---|---|---|---|---|---|---|\n')
    for g, n, t, out, st, notes, small, font, route in rows:
        f.write(f'| `{g}` | {n} | {t} | ' + ' | '.join(out[k][0] for k in ('Mobile','Tablet','Laptop','Desktop')) + f' | {st} | {notes} |\n')
counts = {}
for row in rows: counts[row[4]] = counts.get(row[4], 0) + 1
json.dump({'counts': counts, 'total': len(rows), 'fails': [r[0] for r in rows if any(v[0]=='Fail' for v in r[3].values())], 'warns': [r[0] for r in rows if not any(v[0]=='Fail' for v in r[3].values()) and any(v[0]=='Warn' for v in r[3].values())]}, open('summary.json','w'), indent=1)
print(counts, len(rows))

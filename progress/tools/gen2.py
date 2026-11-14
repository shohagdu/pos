import json, os, re
exec(open('gen.py').read().split("rows = []")[0])  # reuse helpers (generic, ptype, name, grade-like) with before data
after = {r['route']: r for r in json.load(open('after.json'))} if os.path.exists('after.json') else {}
SIZES = [360, 390, 768, 1024, 1366, 1440, 1920]
GROUPS = {'Mobile': [360, 390], 'Tablet': [768, 1024], 'Laptop': [1366], 'Desktop': [1440, 1920]}
WORK = ('/pos', '/products/new', '/products/:id')

def issues(r, g, w):
    m = r['sizes'].get(str(w))
    if not m: return [], []
    fail, warn = [], []
    width = max(m['sw'], m['vw'])
    if width > w + 1: fail.append(f'page is {width}px wide (sideways scroll)')
    if m['inner']: (fail if w >= 1366 else warn).append(f'table scrolls sideways in its frame ({m["inner"][0]})')
    if m.get('cutMoney'): fail.append('amounts cut off with …: ' + ', '.join(m['cutMoney'][:2]))
    if g in WORK and w == 1366 and m['sh'] > m['vh'] + 1: warn.append(f'scrolls at 1366×768 ({m["sh"]}px)')
    return fail, warn

def verdict(r, g, ws):
    f, w = [], []
    for x in ws:
        a, b = issues(r, g, x); f += [f'{x}: {i}' for i in a]; w += [f'{x}: {i}' for i in b]
    return ('Fail' if f else 'Warn' if w else 'Pass'), f + w

# Page-specific notes (shared fixes apply to every page; see shared-components.md).
SPECIFIC = {
 '/pos': 'Products/Cart switch + sticky total/Pay bar below lg; customer & discount bottom sheet (F4/F7 open it); 44px targets; serial-number badges; product images setting also covers cart and search. Files: features/pos/*.',
 '/sales': 'Hand-made cards below xl, filters fold behind a Filters button below lg, Paid/Cashier columns from 2xl. Files: features/sales/SalesPage.tsx.',
 '/sales/:id': 'Hand-made item and payment cards below lg, totals under payments below xl, unit cost column from xl, "Print without returned items". Files: features/sales/SaleDetailPage.tsx, Receipt.tsx, returnMath.ts.',
 '/products': 'Hand-written table moved to the shared DataTable (cards below lg, barcode action in the card footer); search field no longer forces 240px; tabs use the shared Tabs. Files: features/products/ProductsPage.tsx, CatalogNav.tsx.',
 '/products/new': 'Was 46px too wide at every size: hidden ⓘ tooltips (InfoTip) kept their width. Sticky save bar margins follow the new page padding. Files: components/ui/InfoTip.tsx, features/products/ProductFormPage.tsx.',
 '/products/:id': 'Same as /products/new.',
 '/sales/:id/return': 'Line table uses table-stack (one card per line with labelled sold / returnable / quantity / condition / refund). Files: features/sales/SaleReturnPage.tsx.',
 '/purchases/:id/return': 'Line table uses table-stack. Files: features/purchases/PurchaseReturnPage.tsx.',
 '/accounts': 'Payment methods table uses table-stack. Files: features/money/PaymentMethodsSection.tsx.',
 '/accounts/:id': 'Account book uses table-stack (opening balance and totals stay label + amount). Files: features/money/AccountBookPage.tsx.',
 '/customers/:id': 'Statement table uses table-stack; name is the card title in lists. Files: features/customers/CustomerTabs.tsx, CustomersPage.tsx.',
 '/customers': 'Card title is the name, balance on the right. Files: features/customers/CustomersPage.tsx.',
 '/audit': 'Audit table uses table-stack; expand button 44px. Files: features/audit/AuditLogPage.tsx.',
 '/inventory': 'Quantity is the card headline; table becomes cards whenever it does not fit (1024 with sidebar). Files: features/inventory/StockPage.tsx.',
 '/inventory/adjustments/new': 'Line table uses table-stack (once lines are added). Files: features/inventory/AdjustmentFormPage.tsx.',
 '/': 'KPI amounts wrap instead of "…", 6 per row only from 2xl; lists in half-width panels become cards when they do not fit. Files: features/dashboard/widgets.tsx, DashboardPage.tsx.',
 '/users': 'Tabs use the shared Tabs. Files: features/users/UsersPage.tsx.',
 '/labels': 'Grid got a grid-cols-1 base (was 11px too wide at 360). Files: features/labels/LabelPrintPage.tsx.',
}

def label_of(route):
    s = re.sub(r'[?].*$', '', route).lstrip('/')
    s = re.sub(r'[/:]+', '-', s) or 'dashboard'
    for k, v in ids.items():
        if v: s = s.replace(str(v), k)
    return s

rows = []
for r in before:
    route = r['route']; g = generic(route); a = after.get(route)
    bt = {k: verdict(r, g, ws) for k, ws in GROUPS.items()}
    at = {k: verdict(a, g, ws) for k, ws in GROUPS.items()} if a else None
    had = any(v[0] != 'Pass' for v in bt.values())
    still = at and any(v[0] != 'Pass' for v in at.values())
    if g in ('/pos', '/sales', '/sales/:id'): st = 'Fixed'
    elif not at: st = 'Pending'
    elif still: st = 'Fixing'
    else: st = 'Fixed' if had else 'Passed'
    show = at or bt
    landed = (a or r)['sizes']['1366']['path']
    notes = SPECIFIC.get(g, '').split(' Files:')[0]
    if landed.split('?')[0] != route.split('?')[0]: notes = (notes + f' Redirects to {landed}.').strip()
    if still: notes = (notes + ' Still: ' + '; '.join(i for v in at.values() for i in v[1])[:200]).strip()
    rows.append((g, name(g), ptype(g), show, st, notes))
    sm = lambda d, w: d['sizes'][str(w)]['small'] if d else '-'
    lines = [f'# {g}', '', f'- Page type: {ptype(g)}', f'- Audited URL: `{route}` (Bengali)', f'- Status: **{st}**', '', '## Issues found (before)', '']
    for k, (v, iss) in bt.items():
        lines.append(f'- **{k}**: {v}')
        lines += [f'  - {i}' for i in iss]
    lines += [f'- Tap targets under 40px at 360px (incl. app header): {sm(r,360)}; inputs under 16px at 390px: {r["sizes"]["390"]["smallFont"]}',
              '- App-wide (fixed in shared components): sidebar took ~260px on tablets, 36px header buttons, 14px inputs (iOS zoom), wrapping tabs.', '',
              '## Changes', '', '- Shared: app shell drawer below lg, 44px header targets, 16px inputs below lg, scrolling tabs, Dialog bottom sheet on phones, Card padding, DataTable automatic cards, grid-cols-1 bases, InfoTip (see `../shared-components.md`).']
    if g in SPECIFIC: lines.append('- Page: ' + SPECIFIC[g])
    lines += ['', '## Verification (after)', '']
    if at:
        for k, (v, iss) in at.items():
            lines.append(f'- **{k}**: {v}')
            lines += [f'  - {i}' for i in iss]
        lines.append(f'- Tap targets under 40px at 360px: {sm(a,360)} (before {sm(r,360)}); inputs under 16px at 390px: {a["sizes"]["390"]["smallFont"]} (before {r["sizes"]["390"]["smallFont"]})')
        lines.append(f'- Screenshots: `../screenshots/before/` and `../screenshots/after/` (file prefix `{label_of(route)}`)')
    else:
        lines.append('_Not yet._')
    open(f'{P}/pages/{name(g)}.md', 'w').write('\n'.join(lines) + '\n')

with open(f'{P}/audit.md', 'w') as f:
    f.write('# Responsive audit\n\nChecked in Bengali at 360, 390 (mobile), 768, 1024 (tablet), 1366×768 (laptop), 1440, 1920 (desktop). '
            'Device columns show the **current** result (after fixes when verified; per-page files keep the before/after detail).\n\n'
            '**Fail**: page scrolls sideways, a table scrolls sideways on laptop/desktop, or amounts cut off with "…". '
            '**Warn**: a table scrolls sideways inside its frame on a phone/tablet, or a work screen scrolls at 1366×768.\n\n')
    f.write('| Route | Page name | Page type | Mobile | Tablet | Laptop | Desktop | Status | Notes |\n|---|---|---|---|---|---|---|---|---|\n')
    for g, n, t, show, st, notes in rows:
        f.write(f'| `{g}` | [{n}](pages/{n}.md) | {t} | ' + ' | '.join(show[k][0] for k in GROUPS) + f' | {st} | {notes} |\n')
c = {}
for row in rows: c[row[4]] = c.get(row[4], 0) + 1
json.dump({'counts': c, 'total': len(rows), 'open': [r[0] for r in rows if r[4] in ('Fixing', 'Pending')]}, open('summary2.json', 'w'), indent=1)
print(c)

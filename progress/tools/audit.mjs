// node audit.mjs <outdir> <shotdir|-> [routeFilter regex]
import { spawn } from 'node:child_process'
import { writeFileSync, mkdirSync } from 'node:fs'
const [out, shotDir, filt] = process.argv.slice(2)
const W = process.env.SIZES ? process.env.SIZES.split(',').map(x => x.split('x').map(Number)) : [[360,640],[390,844],[768,1024],[1024,768],[1366,768],[1440,900],[1920,1080]]
const port = 9334
const sleep = ms => new Promise(r=>setTimeout(r,ms))
const chrome = spawn('google-chrome', ['--headless=new','--disable-gpu','--hide-scrollbars',`--remote-debugging-port=${port}`,'--user-data-dir=./chrome-audit','about:blank'],{stdio:'ignore'})
let tabs; for (let i=0;i<50;i++){ try{ tabs = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); break }catch{ await sleep(200)} }
const ws = new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl); await new Promise(r=>ws.onopen=r)
let id=0; const pend=new Map(); ws.onmessage=e=>{const m=JSON.parse(e.data); if(m.id&&pend.has(m.id)){pend.get(m.id)(m);pend.delete(m.id)}}
const send=(method,params={})=>new Promise(r=>{const i=++id;pend.set(i,r);ws.send(JSON.stringify({id:i,method,params}))})
const ev = async (expr)=> (await send('Runtime.evaluate',{expression:expr,awaitPromise:true,returnByValue:true})).result?.result?.value
await send('Page.enable')
const base='http://localhost:5173'
const go = async (p, wait=2500) => { await send('Page.navigate',{url:base+p}); await sleep(wait) }
const setVp = (w,h) => send('Emulation.setDeviceMetricsOverride',{width:w,height:h,deviceScaleFactor:1,mobile:w<1024})
await setVp(1366,768)
// Bengali everywhere (the app keeps the language in localStorage).
await go('/login'); await ev(`localStorage.setItem('pos.lang','bn'); for (const k of Object.keys(localStorage)) if (/lang/i.test(k)) localStorage.setItem(k,'bn')`)
await go('/login')
const guestRoutes = ['/login','/register','/forgot-password','/reset-password?token=x','/verify?phone=01700000001','/invite?token=x']
const MEASURE = `(() => {
  const vw = innerWidth, vh = innerHeight
  const doc = document.documentElement
  const scrollers = el => { for (let p = el.parentElement; p; p = p.parentElement) { const s = getComputedStyle(p); if ((s.overflowX==='auto'||s.overflowX==='scroll'||s.overflowX==='hidden') && p !== document.body && p !== doc) return true } return false }
  const vis = el => { const r = el.getBoundingClientRect(); const s = getComputedStyle(el); return r.width>0 && r.height>0 && s.visibility!=='hidden' }
  const over = [...document.querySelectorAll('body *')].filter(el => { if (!vis(el)) return false; const r = el.getBoundingClientRect(); return r.right > vw + 1 && !scrollers(el) && getComputedStyle(el).position!=='fixed' })
    .filter(el => ![...el.children].some(c => c.getBoundingClientRect().right > vw + 1)).slice(0,5)
    .map(el => el.tagName.toLowerCase() + (el.className && typeof el.className==='string' ? '.' + el.className.split(' ').slice(0,3).join('.') : '') + ' →' + Math.round(el.getBoundingClientRect().right))
  const inner = [...document.querySelectorAll('*')].filter(el => { const s = getComputedStyle(el); return (s.overflowX==='auto'||s.overflowX==='scroll') && el.scrollWidth > el.clientWidth + 2 && vis(el) && !el.matches('[role=group], [role=tablist], nav, nav *') && el.querySelector('table') }).map(el => el.scrollWidth + '/' + el.clientWidth)
  const small = [...document.querySelectorAll('button, a[href], input:not([type=hidden]), select, [role=tab]')].filter(el => { if (!vis(el)) return false; const r = el.getBoundingClientRect(); if (r.bottom < 0 || r.top > doc.scrollHeight) return false; if (el.matches('input[type=checkbox], input[type=radio]')) return false; if (el.closest('table') && el.tagName==='A') return false; return r.height < 40 }).length
  const smallFont = [...document.querySelectorAll('input:not([type=checkbox]):not([type=radio]):not([type=hidden]), select, textarea')].filter(el => vis(el) && parseFloat(getComputedStyle(el).fontSize) < 16).length
  const cut = [...document.querySelectorAll('body *')].filter(el => { if (!vis(el) || el.children.length) return false; const s = getComputedStyle(el); return (s.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1) || (s.webkitLineClamp !== 'none' && s.webkitLineClamp && el.scrollHeight > el.clientHeight + 2) }).map(el => el.textContent.trim().slice(0,30))
  const cutMoney = cut.filter(t => /[0-9০-৯৳€]/.test(t))
  return { cut: cut.length, cutMoney: cutMoney.slice(0,4), sw: doc.scrollWidth, sh: doc.scrollHeight, vw, vh, over, inner, small, smallFont, path: location.pathname + location.search, title: document.title, h1: document.querySelector('h1')?.textContent?.trim().slice(0,60) ?? '' }
})()`
const results = []
async function audit(route, label) {
  if (filt && !new RegExp(filt).test(route)) return
  await setVp(1366,768); await go(route)
  const row = { route, label, sizes: {} }
  for (const [w,h] of W) {
    await setVp(w,h); await sleep(700)
    const m = await ev(MEASURE)
    row.sizes[w] = m
    if (shotDir !== '-') {
      const r = await send('Page.captureScreenshot',{format:'jpeg',quality:55})
      if (r.result) writeFileSync(`${shotDir}/${label}-${w}.jpg`, Buffer.from(r.result.data,'base64'))
    }
  }
  results.push(row)
  console.log(route, Object.entries(row.sizes).map(([w,m]) => `${w}:${Math.max(m.sw,m.vw)>Number(w)+1?'X':''}${m.cutMoney.length?'C':''}${m.over.length?'O':''}${m.inner.length?'T':''}`).join(' '), '→', Object.values(row.sizes).at(-1).path)
  writeFileSync(out, JSON.stringify(results, null, 1))
}
const slug = r => (r.replace(/[?].*$/,'').replace(/^\//,'').replace(/[/:]+/g,'-') || 'dashboard')
for (const r of guestRoutes) await audit(r, slug(r))
// sign in
await go('/login')
await ev(`(async()=>{const set=(el,v)=>{const s=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;s.call(el,v);el.dispatchEvent(new Event('input',{bubbles:true}))};
 const ins=[...document.querySelectorAll('input')]; set(ins.find(i=>i.type==='tel'||i.name==='phone'||i.autocomplete==='username')||ins[0],'01700000001'); set(document.querySelector('input[type=password]'),'Test@12345');
 document.querySelector('form button[type=submit]').click()})()`)
await sleep(4000)
await audit('/choose-business', 'choose-business')
if ((await ev('location.pathname'))==='/choose-business'){ await ev(`(()=>{const b=[...document.querySelectorAll('main button, main a, button')].find(x=>/test shop/i.test(x.textContent)); b?.click()})()`); await sleep(3000) }
// ids from the API
const ids = await ev(`(async () => {
  const tok = (await (await fetch('/api/v1/auth/refresh',{method:'POST',credentials:'include'})).json()).access_token
  const get = async (p) => { try { const r = await fetch(p,{headers:{Authorization:'Bearer '+tok}}); const j = await r.json(); const a = Array.isArray(j) ? j : (j.data ?? j.items ?? j.accounts ?? []); return a } catch { return [] } }
  const first = async (p, pick = a => a[0]) => { const a = await get(p); const x = pick(a); return x ? (x.id ?? x.user_id ?? x.membership_id ?? x.tenant_id) : null }
  return {
    sale: await first('/api/v1/sales?limit=20', a => a.find(s => s.status === 'partially_returned') ?? a[0]),
    saleReturn: await first('/api/v1/sale-returns?limit=1'),
    purchase: await first('/api/v1/purchases?limit=1'),
    purchaseReturn: await first('/api/v1/purchase-returns?limit=1'),
    supplier: await first('/api/v1/suppliers?limit=1'),
    customer: await first('/api/v1/customers?limit=5', a => a.find(c => !c.is_walk_in) ?? a[0]),
    product: await first('/api/v1/products?limit=1'),
    adjustment: await first('/api/v1/stock-adjustments?limit=1'),
    transfer: await first('/api/v1/stock-transfers?limit=1'),
    count: await first('/api/v1/stock-counts?limit=1'),
    shift: await first('/api/v1/shifts?limit=1'),
    account: await first('/api/v1/payment-accounts'),
    recon: await first('/api/v1/bank-reconciliations?limit=1'),
    journal: await first('/api/v1/journals?limit=1'),
    member: await first('/api/v1/members'),
    role: await first('/api/v1/roles'),
    tenant: await first('/api/v1/admin/tenants?limit=1'),
  }
})()`)
console.log('ids', JSON.stringify(ids))
writeFileSync(out.replace(/\.json$/,'-ids.json'), JSON.stringify(ids))
const R = (p, key) => key && !ids[key] ? null : p.replace(':id', ids[key] ?? '')
const appRoutes = [
 '/', '/pos', '/settings/regional', '/settings/branch', '/settings/invoice', '/settings/codes', '/settings/branches', '/settings/registers', '/settings/data',
 '/account/sessions','/account/password','/account/notifications','/notifications','/plan',
 '/users','/users/invitations','/users/roles','/users/invite', R('/users/members/:id','member'), '/users/roles/new', R('/users/roles/:id','role'),
 '/products','/products/new','/products/import', R('/products/:id','product'), '/products/setup/categories','/products/setup/brands','/products/setup/units','/products/setup/tax-rates','/labels',
 '/sales','/sales/returns', R('/sales/returns/:id','saleReturn'), '/sales/warranties', R('/sales/:id','sale'), R('/sales/:id/return','sale'),
 '/purchases','/purchases/new','/purchases/returns', R('/purchases/returns/:id','purchaseReturn'), R('/purchases/:id','purchase'), R('/purchases/:id/edit','purchase'), R('/purchases/:id/return','purchase'),
 '/suppliers','/suppliers/new', R('/suppliers/:id','supplier'), R('/suppliers/:id/edit','supplier'),
 '/customers','/customers/new', R('/customers/:id','customer'), R('/customers/:id/edit','customer'),
 '/inventory','/inventory/movements','/inventory/expiry','/inventory/adjustments','/inventory/adjustments/new', R('/inventory/adjustments/:id','adjustment'),
 '/inventory/transfers','/inventory/transfers/new', R('/inventory/transfers/:id','transfer'), '/inventory/counts', R('/inventory/counts/:id','count'),
 '/shifts', R('/shifts/:id','shift'), '/accounts','/accounts/reconciliations', R('/accounts/reconciliations/:id','recon'), R('/accounts/:id','account'),
 '/expenses','/expenses/categories','/incomes','/incomes/categories',
 '/reports','/reports/sales','/reports/purchases','/reports/inventory','/reports/expenses','/reports/payments','/reports/profit','/reports/trial-balance','/reports/profit-and-loss','/reports/balance-sheet','/reports/general-ledger','/reports/aging','/reports/customer-dues','/reports/supplier-dues','/reports/vat','/reports/discounts','/reports/product-movement',
 '/accounting/journals', R('/accounting/journals/:id','journal'), '/accounting/periods','/audit',
 '/onboarding/business','/onboarding/account','/onboarding/regional','/onboarding/branch','/onboarding/invoice',
 '/admin','/admin/tenants', R('/admin/tenants/:id','tenant'), '/admin/plans','/admin/claims','/admin/settings',
].filter(Boolean)
const label = r => { const s = slug(r); return Object.entries(ids).reduce((acc,[k,v]) => v ? acc.replace(String(v), ':' + k) : acc, s).replace(/:/g,'') }
for (const r of appRoutes) await audit(r, label(r))
ws.close(); chrome.kill()

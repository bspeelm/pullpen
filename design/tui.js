(function(){
const PAL={
 tc:{bg:'#141517',surf:'#1e1f23',sel:'#282b31',fg:'#cbc9c4',dim:'#8a8984',faint:'#505055',acc:'#7cc4dc',ok:'#94c98b',err:'#e48a7a',warn:'#dcc07a'},
 c16:{bg:'#141517',surf:'transparent',sel:'transparent',fg:'#c0c0c0',dim:'#808080',faint:'#808080',acc:'#33b8c8',ok:'#4fb84f',err:'#e04a40',warn:'#d0aa20'}
};
const ASCII={'✓':'+','✗':'x','○':'o','≠':'!','◌':'-','◆':'*','◇':'^','●':'.','▌':'>','│':'|','…':'~','─':'-','•':'+','→':'>','←':'<','›':'>','↑':'^','↓':'v','⠹':'/','·':'-','−':'-','–':'-'};
const AG=['claude','codex','renovate','dependabot','devin'];
const isAg=p=>AG.includes(p.a);
const S=(t,f='fg',o={})=>({t:String(t),f,k:o.k||null,b:!!o.b,u:!!o.u});
const len=l=>l.reduce((n,s)=>n+[...s.t].length,0);
const sp=n=>S(' '.repeat(Math.max(0,n)));
function fit(l,w,ell=true){
 const n=len(l); if(n<=w) return n<w?[...l,sp(w-n)]:l.slice(); if(w<=0) return [];
 const lim=ell?w-1:w; const out=[]; let c=0;
 for(const s of l){const cs=[...s.t]; if(c+cs.length<=lim){out.push(s);c+=cs.length;} else {const k=lim-c; if(k>0) out.push({...s,t:cs.slice(0,k).join('')}); c=lim; break;}}
 if(ell){ let last=out[out.length-1]||S(''); let r=0; if(last.t.endsWith(' ')){const tr=last.t.replace(/ +$/,''); r=last.t.length-tr.length; out[out.length-1]={...last,t:tr};}
  out.push({...last,t:'…',u:false}); if(r) out.push(sp(r)); }
 return out;
}
const withBg=(l,k)=>l.map(s=>s.k?s:{...s,k});
const lr=(a,b,w)=>[...fit(a,w-len(b)),...b];
function M(str){
 const T={f:'fg',d:'dim',x:'faint',a:'acc',g:'ok',r:'err',y:'warn'};
 const out=[],re=/\{([a-zA-Z]+)\|([^}]*)\}/g; let i=0,m;
 while((m=re.exec(str))){ if(m.index>i) out.push(S(str.slice(i,m.index)));
  let f='fg',b=false,u=false,k=null;
  for(const c of m[1]){ if(T[c]) f=T[c]; else if(c==='B') b=true; else if(c==='U') u=true; else if(c==='C'){k='fg';f='bg';} else if(c==='S') k='sel'; }
  out.push(S(m[2],f,{b,u,k})); i=re.lastIndex; }
 if(i<str.length) out.push(S(str.slice(i))); return out;
}
function wrap(t,w){const out=[];let cur='';for(const wd of t.split(' ')){if(!cur)cur=wd;else if((cur+' '+wd).length<=w)cur+=' '+wd;else{out.push(cur);cur=wd;}}if(cur)out.push(cur);return out;}
function toCells(l){const c=[];for(const s of l)for(const ch of s.t)c.push({...s,t:ch});return c;}
function fromCells(c){const o=[];for(const x of c){const p=o[o.length-1];if(p&&p.f===x.f&&p.k===x.k&&p.b===x.b&&p.u===x.u)p.t+=x.t;else o.push({...x});}return o;}
function overlay(base,x,top){const c=toCells(base),t=toCells(top);for(let i=0;i<t.length;i++)c[x+i]=t[i];return fromCells(c);}

// ---------- data
const P=[
 {f:'gh',r:'acme/api',n:4821,t:'Migrate invoices to ledger v2',a:'claude',ci:'ok',rv:[1,2],armed:1,ad:412,de:88,age:'3h'},
 {f:'gl',r:'platform/runner',n:812,t:'Bump go toolchain to 1.25.3',a:'renovate',ci:'ok',rv:[1,1],ad:6,de:6,age:'41m'},
 {f:'gh',r:'acme/web',n:2290,t:'Fix hydration mismatch on settings page when locale is RTL',a:'codex',ci:'fail',rv:[0,1],armed:1,ad:38,de:12,age:'5h'},
 {f:'gh',r:'acme/infra',n:377,t:'Rotate staging TLS certs via cert-manager',a:'claude',ci:'run',rv:[0,1],ad:54,de:20,age:'12m'},
 {f:'gl',r:'platform/charts',n:145,t:'Add PodDisruptionBudget to ingest workers',a:'claude',ci:'ok',rv:[1,1],ad:31,de:0,age:'1h'},
 {f:'gh',r:'acme/api',n:4818,t:'Add idempotency keys to POST /payments',a:'codex',ci:'ok',rv:[2,2],conflict:1,armed:1,ad:220,de:41,age:'1d'},
 {f:'gh',r:'tools/rq',n:96,t:'Cache forge capabilities per host',a:'claude',ci:'ok',rv:[1,1],ad:73,de:15,age:'2h'},
 {f:'gh',r:'acme/web',n:2288,t:'chore(deps): update dependency vite to v7.1.4',a:'dependabot',ci:'ok',rv:[0,1],armed:1,ad:3,de:3,age:'6h'},
 {f:'gl',r:'platform/runner',n:809,t:'Retry artifact upload on 503 with jittered backoff',a:'codex',ci:'run',rv:[1,1],armed:1,ad:64,de:9,age:'30m',b:'develop'},
 {f:'gh',r:'acme/billing',n:1203,t:'Proration preview endpoint',a:'claude',ci:'none',draft:1,rv:[0,2],ad:388,de:2,age:'2d'},
 {f:'gh',r:'acme/api',n:4815,t:'Remove deprecated v1 webhooks handler',a:'claude',ci:'ok',rv:[2,2],ad:12,de:540,age:'2d'},
 {f:'gl',r:'platform/charts',n:143,t:'Pin redis chart to 19.6.x',a:'renovate',ci:'fail',rv:[0,1],ad:2,de:2,age:'3d'},
 {f:'gh',r:'acme/infra',n:375,t:'Terraform: split vpc module by region',a:'codex',ci:'ok',rv:[0,1],ad:610,de:402,age:'3d'},
 {f:'gh',r:'acme/billing',n:1199,t:'Ledger: emit backfill progress metrics',a:'claude',ci:'ok',rv:[1,1],armed:1,ad:47,de:3,age:'4h'},
 {f:'gl',r:'platform/runner',n:806,t:'Drop support for docker-machine executor',a:'devin',ci:'ok',rv:[0,2],ad:12,de:1340,age:'4d'},
 {f:'gh',r:'acme/web',n:2284,t:'Expose ledger balances in admin account view',a:'codex',ci:'run',rv:[0,1],ad:156,de:22,age:'7h'},
 {f:'gh',r:'acme/api',n:4809,t:'Bump pgx to v5.7.2',a:'renovate',ci:'ok',rv:[1,1],armed:1,ad:4,de:4,age:'5d'},
 {f:'gl',r:'platform/docs',n:58,t:'Document runner autoscaling flags',a:'claude',ci:'ok',rv:[0,1],ad:88,de:10,age:'5d'},
 {f:'gh',r:'tools/rq',n:94,t:'Map GitLab approval rules to required reviews',a:'claude',ci:'ok',rv:[1,1],ad:132,de:40,age:'6d'},
 {f:'gh',r:'acme/infra',n:371,t:'Enable VPC flow logs to S3 with 30-day retention',a:'devin',ci:'fail',rv:[0,1],ad:41,de:0,age:'6d'},
 {f:'gh',r:'acme/web',n:2279,t:'chore(deps): update dependency @types/node to v22.18',a:'dependabot',ci:'ok',rv:[0,1],ad:2,de:2,age:'1w'},
 {f:'gl',r:'platform/charts',n:139,t:'Bump ingress-nginx to 4.13.1',a:'renovate',ci:'ok',rv:[1,1],ad:2,de:2,age:'1w'},
 {f:'gh',r:'acme/api',n:4796,t:'Trace context propagation for outbound HTTP',a:'codex',ci:'ok',rv:[0,1],ad:95,de:18,age:'2w'},
];
const H=[
 {f:'gh',r:'acme/api',n:4823,t:'Rate limit per API key instead of per IP',a:'mara',ci:'run',rv:[0,1],ad:140,de:31,age:'20m'},
 {f:'gh',r:'acme/web',n:2291,t:'New billing settings layout',a:'jkoh',ci:'ok',rv:[1,1],ad:820,de:615,age:'1h'},
 {f:'gl',r:'platform/runner',n:814,t:'Ledger exporter for runner minutes',a:'ash',ci:'fail',rv:[0,1],ad:203,de:14,age:'2h'},
 {f:'gh',r:'acme/billing',n:1205,t:'Invoice PDF: handle multi-currency totals',a:'priya',ci:'ok',rv:[0,2],ad:96,de:30,age:'4h'},
 {f:'gl',r:'platform/docs',n:60,t:'Fix broken links in runner install guide',a:'lena',ci:'ok',rv:[1,1],ad:9,de:9,age:'5h'},
 {f:'gh',r:'acme/infra',n:378,t:'Grafana: alert on ledger write latency p99',a:'ash',ci:'ok',rv:[0,1],ad:58,de:0,age:'8h'},
 {f:'gh',r:'acme/api',n:4811,t:'Split payments service config by environment',a:'mara',ci:'ok',rv:[2,2],ad:77,de:65,age:'1d'},
 {f:'gl',r:'platform/charts',n:146,t:'Allow custom annotations on runner pods',a:'tomasz',ci:'run',rv:[0,1],ad:22,de:3,age:'1d'},
 {f:'gh',r:'acme/web',n:2286,t:'Keyboard shortcuts for the invoice table',a:'jkoh',ci:'ok',rv:[0,1],draft:1,ad:140,de:12,age:'2d'},
 {f:'gh',r:'tools/rq',n:97,t:'Command-line history persists across sessions',a:'ash',ci:'ok',rv:[0,0],ad:66,de:8,age:'2d'},
 {f:'gh',r:'acme/api',n:4802,t:'Upgrade to Go 1.25',a:'mara',ci:'fail',rv:[0,1],ad:31,de:29,age:'3d'},
 {f:'gl',r:'platform/runner',n:801,t:'Shell executor: honor umask from config',a:'lena',ci:'ok',rv:[1,1],ad:15,de:2,age:'1w'},
];
const ALL=(()=>{const o=[];let i=0,j=0;while(i<P.length||j<H.length){if(j<H.length)o.push(H[j++]);if(i<P.length)o.push(P[i++]);if(i<P.length)o.push(P[i++]);}return o;})();
const byN=n=>P.find(p=>p.n===n)||H.find(p=>p.n===n);

// ---------- row A (default)
const PL={run:S('○','warn'),ok:S('✓','ok'),fail:S('✗','err'),fixed:S('✗','dim')};
function stateG(p){ if(p.merged) return PL[p.pl]||S('●','dim'); if(p.draft) return S('◌','dim'); if(p.conflict) return S('≠','err'); if(p.ci==='fail') return S('✗','err'); if(p.ci==='run'||p.rv[0]<p.rv[1]) return S('○','warn'); return S('✓','ok'); }
function revS(p){const [h,n]=p.rv; if(!n) return S('–','faint'); return S(h+'/'+n,h>=n?'ok':'warn');}
function cols(w){
 const c={forge:3,repo:17,target:10,author:12,rev:5,size:12,age:4}; const sum=()=>12+c.forge+c.repo+c.target+c.author+c.rev+c.size+c.age;
 // The target branch folds after size, so it still shows beside the preview
 // on a wide terminal.
 const steps=[()=>c.size=0,()=>c.target=0,()=>c.author=2,()=>c.repo=11,()=>c.age=0,()=>c.rev=0,()=>c.forge=0,()=>c.repo=0];
 for(const st of steps){ if(sum()+24<=w) break; st(); }
 c.title=w-sum(); return c;
}
function titleSegs(p,hl,f){ if(!hl) return [S(p.t,f)]; const i=p.t.toLowerCase().indexOf(hl); if(i<0) return [S(p.t,f)];
 return [S(p.t.slice(0,i),f),S(p.t.slice(i,i+hl.length),'acc',{u:1}),S(p.t.slice(i+hl.length),f)]; }
function gut(o){return o.sel?S('▌ ','acc'):o.mark?S('• ','acc'):sp(2);}
function rowA(p,w,o={}){
 const c=cols(w),sel=o.sel; let l=[gut(o)];
 if(c.forge) l.push(S(p.f.padEnd(c.forge),'dim'));
 l.push(o.spin===p.n?S('⠹','acc'):stateG(p),sp(1),p.armed?S('◆','acc'):sp(1),sp(1));
 l.push(S(('#'+p.n).padStart(5)+' ',sel?'fg':'dim'));
 const tf=p.draft||p.merged?'dim':'fg';
 l.push(...fit(titleSegs(p,o.hl,tf).map(s=>sel&&s.f==='fg'?{...s,b:true}:s),c.title-2),sp(2));
 if(c.repo) l.push(...fit([S(c.repo===11?p.r.split('/').pop():p.r,'dim')],c.repo-1),sp(1));
 if(c.target) l.push(...fit([S(p.b||'main','dim')],c.target-1),sp(1));
 if(c.author===12) l.push(...fit([isAg(p)?S('◇ ','fg'):sp(2),S(p.a,'dim')],11),sp(1)); else if(c.author===2) l.push(isAg(p)?S('◇ ','fg'):sp(2));
 if(c.rev) l.push(...fit([revS(p)],c.rev-1),sp(1));
 if(c.size) l.push(S(('+'+p.ad).padStart(5)+' '+('−'+p.de).padEnd(5)+' ','dim'));
 if(c.age) l.push(S(p.age.padStart(4),'dim'));
 l=fit(l,w,false);
 if(o.dimAll) l=l.map(s=>({...s,f:'faint',b:false}));
 if(sel) l=withBg(l,'sel');
 return l;
}
function headA(w){
 const c=cols(w),l=[sp(2)]; if(c.forge) l.push(sp(c.forge));
 l.push(sp(4),S('    #','faint'),sp(1),...fit([S('title','faint')],c.title-2),sp(2));
 if(c.repo) l.push(...fit([S('repo','faint')],c.repo));
 if(c.target) l.push(...fit([S('target','faint')],c.target));
 if(c.author===12) l.push(...fit([sp(2),S('author','faint')],12)); else if(c.author===2) l.push(sp(2));
 if(c.rev) l.push(...fit([S('rev','faint')],c.rev));
 if(c.size) l.push(...fit([S('  +/−','faint')],c.size));
 if(c.age) l.push(S(' age','faint'));
 return fit(l,w,false);
}

// ---------- chrome
const SECS=[['Mine','Mine',6],['Needs my review','Review',4],['Auto-merge armed','Armed',7],['Deploys','Deploys',2],['Merged','Merged',7],['Open','Open',96]];
function tabs(active,W,o={}){
 const l=[sp(2)];
 SECS.forEach((s,i)=>{const nm=o.short?s[1]:s[0];
  if(i===active) l.push(S(nm,'acc',{b:1}),S(' '+s[2],'acc')); else l.push(S(nm,'dim'),S(' '+s[2],'faint'));
  if(i<SECS.length-1) l.push(sp(o.short?3:4));});
 return lr(l,o.right?[S(o.right,'faint'),sp(1)]:[],W);
}
function status(W,o){
 const left=[S(' '+o.mode+' ','bg',{k:o.modeK||'acc',b:1}),sp(2),S(o.section,'fg'),sp(2),S(o.pos||'','dim')];
 if(o.extra) left.push(sp(3),...o.extra);
 const r=[]; const ar=o.armed??7, fl=o.failing??1, br=o.broken??1;
 if(o.err) r.push(S('E ','err',{b:1}),S(o.err,'err'),sp(3));
 if(o.narrow) r.push(S('◆','acc'),S(''+ar,'dim'),sp(1),S('✗','err'),S(''+fl,'dim'),...(br?[sp(1),S('✗','err'),S(br+' broken','dim')]:[]),sp(3));
 else r.push(S('◆ ','acc'),S(ar+' armed','dim'),sp(3),S('✗ ','err'),S(fl+' failing','dim'),sp(3),...(br?[S('✗ ','err'),S(br+' broken','dim'),sp(3)]:[]));
 if(!o.err) r.push(S(o.sync||'synced 40s','dim'),sp(3));
 r.push(S('gh','dim'),S(' ✓','ok'),sp(2),S('gl','dim'),o.err?S(' ✗','err'):S(' ✓','ok'),sp(1));
 return withBg(lr(left,r,W),'surf');
}
function listScreen(o){
 const W=o.W,Hh=o.H,L=[]; L.push(tabs(o.tab,W,{short:o.short,right:o.right})); L.push([]);
 const bodyH=Hh-4, pw=46, lw=o.preview?W-pw-4:W-1;
 const rows=[headA(lw),...o.prs.slice(0,bodyH-1).map((p,i)=>rowA(p,lw,{sel:i===o.sel,mark:o.marks&&o.marks.includes(p.n),hl:o.hl,dimAll:o.hl&&!p.t.toLowerCase().includes(o.hl)}))];
 for(let i=0;i<bodyH;i++){const r=rows[i]||[];
  if(o.preview) L.push([...fit(r,lw,false),S(' │ ','faint'),...fit(o.preview[i]||[],pw),sp(1)]); else L.push(fit(r,W,false));}
 L.push(status(W,o.st)); L.push(fit(o.cmd||[],W)); return L;
}
function preview4821(w){
 const d='Moves invoice writes onto the ledger v2 double-entry tables. The backfill runs as an online migration in batches of 5,000 with a checkpoint table, so it resumes cleanly after a deploy.';
 return [
  M('{B|Migrate invoices to ledger v2}'),
  M('{d|gh  acme/api #4821   }{f|◇ claude}{d|   3h}'),
  [],
  M('{y|○ waiting}{d|  squash-merges when @mara approves}'),
  [],
  M('{g|✓} {d|checks      }{f|12/12 passed}'),
  M('{y|○} {d|reviews     }{f|1 of 2}{d|   @mara pending}'),
  M('{g|✓} {d|conflicts   }{f|none}{d|   up to date}'),
  M('{a|◆} {d|auto-merge  }{f|armed}{d|   squash · @ash}'),
  [],
  M('{d|claude/ledger-v2 → main}'),
  M('{d|+412 −88 · 14 files}'),
  [],
  ...wrap(d,w).map(t=>[S(t)]),
  [],
  M('{d|activity}'),
  lr(M('{g|✓} {f|jkoh}{d| approved}'),M('{d|2h}'),w),
  lr(M('{a|◆} {f|ash}{d| armed auto-merge}'),M('{d|2h}'),w),
  lr(M('{d|·} {f|◇ claude}{d| pushed 2 commits}'),M('{d|3h}'),w),
  lr(M('{y|○} {f|mara}{d| review requested}'),M('{d|3h}'),w),
 ];
}

// ---------- detail
const DIFF=[
 ['h','@@ -38,14 +38,24 @@ migrateInvoices(db, opts)'],
 [38,38,' ','const cursor = await loadCheckpoint(db)'],
 [39,39,' ','while (true) {'],
 [40,40,' ','  const batch = await nextBatch(cursor, 5000)'],
 [41,'','-','  if (!batch.length) break'],
 [42,'','-','  for (const inv of batch) await writeLegacy(inv)'],
 ['',41,'+','  if (batch.length === 0) {'],
 ['',42,'+','    await markComplete(db)'],
 ['',43,'+','    break'],
 ['',44,'+','  }'],
 ['',45,'+','  await db.tx(async (tx) => {'],
 ['',46,'+','    for (const inv of batch) {'],
 ['',47,'+','      await writeLegacy(tx, inv)'],
 ['',48,'+','      await writeLedger(tx, toEntries(inv))'],
 ['',49,'+','    }'],
 ['',50,'+','    await saveCheckpoint(tx, batch.at(-1).id)'],
 ['',51,'+','  })'],
 [43,52,' ','  metrics.batches.inc()'],
 [44,53,' ','}'],
 ['b'],
 ['h','@@ -88,6 +98,9 @@ function toEntries(inv: Invoice)'],
 [88,98,' ','return ['],
 [89,99,' ','  debit(inv.accountId, inv.total),'],
 [90,100,' ','  credit(LEDGER_REVENUE, inv.net),'],
 ['',101,'+','  ...inv.taxLines.map((t) =>'],
 ['',102,'+','    credit(LEDGER_TAX, t.amount)),'],
 [91,103,' ',']'],
];
function dline(d){ if(d[0]==='b') return []; if(d[0]==='h') return [S(d[1],'dim')];
 const [o,n,s,c]=d; const f=s==='+'?'ok':s==='-'?'err':'fg';
 return [S(String(o).padStart(3)+' '+String(n).padStart(3)+'  ','faint'),S(s,f),S(' '+c,f)]; }
function detailScreen(){
 const W=120,Hh=40,L=[];
 L.push(lr([sp(2),S('Open','dim'),S('  ›  ','faint'),S('gh  acme/api  ','dim'),S('#4821','fg',{b:1})],[S('2 of 96','dim'),sp(4),S('updated 12m ago','dim'),sp(1)],W));
 L.push([]);
 L.push([sp(2),S('Migrate invoices to ledger v2','fg',{b:1})]);
 L.push([sp(2),S('◇ claude','fg'),S('   claude/ledger-v2 → main   +412 −88   14 files   opened 3h ago','dim')]);
 L.push([]);
 L.push([sp(2),S(' WAITING ','bg',{k:'warn',b:1}),sp(3),S('Squash-merges automatically when @mara approves. Nothing else is blocking.','fg')]);
 L.push([]);
 const g=(a,b)=>[sp(2),...fit(M(a),59),...M(b)];
 L.push(g('{g|✓}  {d|checks      }{f|12/12 passed}{d|   8m 14s}','{g|✓}  {d|conflicts   }{f|none}{d|   up to date with main}'));
 L.push(g('{y|○}  {d|reviews     }{f|1 of 2 required}{d|   jkoh ✓   mara pending}','{a|◆}  {d|auto-merge  }{f|armed}{d|   squash · by ash 2h ago}'));
 L.push([]);
 const LW=56,RW=W-2-LW-3-1;
 L.push([sp(2),...fit(lr(M('{aB|overview}'),M('{d|description · checks · conversation}'),LW),LW),S(' │ ','faint'),...fit(lr(M('{B|src/ledger/migrate.ts}'),M('{d|file 2/14   }{g|+120}{d| }{r|−12}'),RW),RW)]);
 const desc='Moves invoice writes onto the ledger v2 double-entry tables. The backfill runs as an online migration in batches of 5,000 with a checkpoint table, so it resumes cleanly after a deploy. Reads switch behind the ledger_v2_reads flag, off by default.';
 const ck=(n,t)=>lr(M('{g|✓} '+n),M('{d|'+t+'}'),LW);
 const left=[
  [],
  ...wrap(desc,LW).map(t=>[S(t)]),
  [],
  M('{d|-} add migrate_invoices job and checkpoint table'),
  M('{d|-} dual-write legacy + ledger until the flag flips'),
  M('{d|-} remove InvoiceTotalsCache, unused since #4790'),
  [],
  lr(M('{d|checks}'),M('{d|12 passed}'),LW),
  ck('build','1m 52s'),ck('test / unit','3m 10s'),ck('test / integration','7m 41s'),ck('lint','38s'),ck('migrations / dry-run','2m 05s'),
  M('{d|  + 7 more passed            zo to expand}'),
  [],
  lr(M('{d|conversation}'),M('{d|4}'),LW),
  lr(M('{g|✓} {f|jkoh}{d|  approved}'),M('{d|2h}'),LW),
  ...wrap('Batching looks right. The checkpoint table needs an index on invoice_id.',LW-4).map(t=>[sp(4),S(t)]),
  lr(M('{d|·} {f|◇ claude}{d|  pushed 1 commit}'),M('{d|1h}'),LW),
  M('    {d|add index on checkpoint(invoice_id)}'),
  lr(M('{y|○} {f|mara}{d|  review requested by claude}'),M('{d|3h}'),LW),
 ];
 const right=[[],...DIFF.map(dline)];
 const bodyH=Hh-L.length-2;
 for(let i=0;i<bodyH;i++) L.push([sp(2),...fit(left[i]||[],LW),S(' │ ','faint'),...fit(right[i]||[],RW),sp(1)]);
 L.push(status(W,{mode:'NORMAL',section:'#4821',pos:'overview'}));
 L.push(M('{d|n next file   b previous file   F all files   d widen the diff   q back to the list}'));
 return L.map(l=>fit(l,W,false));
}

// ---------- help overlay
function helpScreen(){
 const base=listScreen({W:120,H:40,tab:5,prs:ALL,sel:1,preview:preview4821(46),right:'is:open',st:{mode:'NORMAL',section:'Open',pos:'2/96'}});
 const L=base.map((l,i)=>i>=38?l:l.map(s=>({...s,f:'faint',k:null,b:false,u:false})));
 const PW=96, pad=l=>withBg(fit([sp(3),...l],PW),'surf');
 const K=(k,d)=>k?[...fit([S(k,'fg',{b:1})],11),...fit([S(d,'dim')],19)]:sp(30);
 const block=(heads,cs)=>{const out=[pad(heads.flatMap(h=>fit([S(h,'acc')],30)))];
  const n=Math.max(...cs.map(c=>c.length)); for(let i=0;i<n;i++) out.push(pad(cs.flatMap(c=>c[i]?K(...c[i]):[sp(30)]))); return out;};
 const pl=[pad([]),pad(lr(M('{B|keys}'),M('{d|? or Esc to close}'),PW-6)),pad([])];
 pl.push(...block(['move','act on selection','sections & views'],[
  [['j k','down / up'],['gg G','top / bottom'],['^d ^u','half page'],['/','filter list'],['n N','next / prev match'],['Esc','clear, cancel'],['Space','mark row'],['V','mark range'],['Enter','open detail']],
  [['M','merge…'],['A','arm / disarm auto'],['a','approve'],['c','comment'],['d','open diff'],['o','open in browser'],['x','close…'],['u','update branch'],['D','dismiss failure'],['r','refresh']],
  [['h l','prev / next section'],['Tab S-Tab','next / prev section'],['1–6','go to section'],['w','watch pipeline'],['p','toggle preview'],[':','command line'],['?','this help'],['q','back / quit']]]));
 pl.push(pad([]));
 pl.push(...block(['detail view','command line','confirm prompt'],[
  [['h l','focus pane'],['n b','next / prev file'],['F','all files'],[']c [c','next / prev hunk'],['zo zc','expand / fold'],['q Esc','back to list']],
  [['Tab S-Tab','complete'],['↑ ↓','history'],['^r #','insert selected'],['Enter','run'],['Esc','cancel']],
  [['y Enter','confirm'],['n Esc','cancel'],['s m r','pick method'],['','other key cancels']]]));
 pl.push(pad([]));
 pl.push(pad(M('{d|Every key has a : command. M and x confirm on the command line, never in a dialog.}')));
 pl.push(pad([]));
 pl.forEach((l,i)=>{L[5+i]=overlay(L[5+i],12,l);});
 return L;
}

// ---------- strips
function menuPR(p,sel,w){let l=fit([sp(1),stateG(p),sp(1),S(('#'+p.n).padEnd(7),'fg',{b:sel}),...fit([S(p.t,'fg')],40),sp(2),S(p.r,'dim')],w);return sel?withBg(l,'sel'):l;}
function strip(o){
 const W=120; const L=o.prs.map((p,i)=>fit(rowA(p,W-1,{sel:i===o.sel,mark:o.marks&&o.marks.includes(p.n),spin:o.spin}),W,false));
 if(o.menu){const n=o.menu.length;o.menu.forEach((m,i)=>{const ix=L.length-n+i;L[ix]=overlay(L[ix],0,withBg(fit(m,o.menuW||72),'surf'));});}
 // The strips are an Agent-opened section added back in config, which counts
 // its failures too: #2290, #143, #371.
 L.push(status(W,{section:'Agent-opened',mode:'NORMAL',failing:3,...o.st})); L.push(fit(o.cmd||[],W)); return L;
}
const A=(n,x)=>({...byN(n),...x});
function rowArm(p,w,o={}){
 const sel=o.sel,RW=34,tw=w-2-3-2-2-6-12-RW-5;
 let l=[gut(o),S(p.f+' ','dim'),stateG(p),sp(1),p.merged?sp(1):S('◆','acc'),sp(1),S(('#'+p.n).padStart(5)+' ',sel?'fg':'dim'),
  ...fit([S(p.t,p.merged&&!o.lit?'dim':'fg',{b:sel})],tw-2),sp(2),...fit([S(p.r.split('/').pop(),'dim')],11),sp(1),...fit(M(p.why),RW),S(p.when.padStart(5),'dim')];
 l=fit(l,w,false); return sel?withBg(l,'sel'):l;
}
function armedScreen(){
 const W=120,L=[tabs(2,W,{right:'is:armed'}),[]];
 const grp=(t,rows,first)=>{L.push([sp(2),S(t,'fg',{b:1}),S('  '+rows.length,'dim')]);rows.forEach((p,i)=>L.push(rowArm(p,W-1,{sel:first&&i===0})));L.push([]);};
 grp('needs attention',[A(2290,{why:'{r|failing}{d|  e2e/chromium · 2 retries}',when:'18m'}),A(4818,{why:'{r|conflict}{d|  with main after #4815}',when:'25m'})],true);
 grp('waiting',[A(4821,{why:'{y|review}{d|  @mara · 1 of 2}',when:'2h'}),A(809,{why:'{y|ci}{d|  6/9 running · ~4m left}',when:'30m'}),A(377,{armed:1,why:'{y|ci}{d|  4/9 running, then review}',when:'12m'}),A(145,{armed:1,why:'{y|merge train}{d|  position 2 of 3}',when:'1h'}),A(4809,{ci:'run',why:'{y|ci}{d|  required check sbom queued}',when:'5d'})]);
 while(L.length<38) L.push([]);
 L.push(status(W,{mode:'NORMAL',section:'Auto-merge armed',pos:'1/7'}));
 L.push(lr(M('{a|◆} {f|5 merged since 09:14}{d| → Merged   ·   2 need attention   ·   5 still waiting}'),M('{d|Enter detail   A disarm   w pipeline}'),W));
 return L.map(l=>fit(l,W,false));
}

// Deploys: one lane per repo and branch that merges landed on, showing the
// newest merge there that ran a pipeline. A failing lane is what needs you:
// it stays failing until the branch passes again or D dismisses it.
function rowLane(p,w,o={}){
 const sel=o.sel,RW=34,tw=w-2-3-2-12-10-6-RW-5;
 let l=[gut(o),S(p.f+' ','dim'),stateG(p),sp(1),...fit([S(p.r.split('/').pop(),'fg',{b:sel})],11),sp(1),...fit([S(p.b||'main','fg',{b:sel})],9),sp(1),
  S(('#'+p.n).padStart(5)+' ','dim'),...fit([S(p.t,'dim')],tw-2),sp(2),...fit(M(p.why),RW),S(p.when.padStart(5),'dim')];
 l=fit(l,w,false); return sel?withBg(l,'sel'):l;
}
function deploysScreen(){
 const W=120,L=[tabs(3,W,{right:'is:merged merged:7d'}),[]];
 let first=true;
 const grp=(t,rows)=>{L.push([sp(2),S(t,'fg',{b:1}),S('  '+rows.length,'dim')]);rows.forEach(p=>{L.push(rowLane({...p,merged:1},W-1,{sel:first}));first=false;});L.push([]);};
 grp('deploying',[A(2288,{pl:'run',why:'{y|running}{d|  5/7 · deploy-staging}',when:'22m'}),A(812,{pl:'run',why:'{y|running}{d|  2/4 · build}',when:'41m'})]);
 grp('failing',[A(4815,{pl:'fail',why:'{r|main failing}{d|  deploy-production}',when:'1h'})]);
 grp('deployed',[A(1199,{pl:'ok',why:'{g|passed}{d|  6/6}',when:'1h'}),A(4811,{pl:'ok',b:'develop',why:'{g|passed}{d|  11/11}',when:'3h'})]);
 while(L.length<38) L.push([]);
 L.push(status(W,{mode:'NORMAL',section:'Deploys',pos:'1/5'}));
 L.push(lr(M('{r|✗} {f|1 failing}{d|   ·   2 deploying   ·   2 deployed}'),M('{d|Enter pipeline   D dismiss   o open}'),W));
 return L.map(l=>fit(l,W,false));
}

// Merged: what merged this week, newest first, each with the glyph of what
// its merge commit's pipeline did. Deploys is where a failure is acted on.
function mergedScreen(){
 const W=120,L=[tabs(4,W,{right:'is:merged merged:7d'}),[]];
 const rows=[A(2288,{pl:'run',why:'{y|running}{d|  5/7 · deploy-staging}',when:'22m'}),A(812,{pl:'run',why:'{y|running}{d|  2/4 · build}',when:'41m'}),
  A(4815,{pl:'fail',why:'{r|failed}{d|  deploy-production}',when:'1h'}),A(1199,{pl:'ok',why:'{g|passed}{d|  6/6}',when:'1h'}),
  A(4811,{pl:'ok',why:'{g|passed}{d|  11/11}',when:'3h'}),A(60,{why:'{d|no pipeline}',when:'5h'}),A(4802,{pl:'fixed',why:'{g|fixed}{d|  main passes now}',when:'6h'})];
 L.push([sp(2),S('last 7 days','fg',{b:1}),S('  '+rows.length,'dim')]);
 rows.forEach((p,i)=>L.push(rowArm({...p,merged:1,armed:0},W-1,{sel:i===0,lit:1})));
 while(L.length<38) L.push([]);
 L.push(status(W,{mode:'NORMAL',section:'Merged',pos:'1/7'}));
 L.push(lr(M('{d|7 merged}'),M('{d|w pipeline   :clear drop finished}'),W));
 return L.map(l=>fit(l,W,false));
}

// A PR's pipeline is a subscreen of the PR: the PR stays named on top, its
// runs below, newest first, each job with its stage, progress and time. Enter
// opens a job; q walks back to wherever w was pressed.
function prHead(crumb){
 return [
  lr([sp(2),S('Merged','dim'),S('  ›  ','faint'),S('gl  platform/runner  ','dim'),S('#812','fg',{b:1}),S('  ›  ','faint'),S(crumb,'fg')],[S('refreshed 3s ago','dim'),sp(1)],120),
  [],
  [sp(2),S('Bump go toolchain to 1.25.3','fg',{b:1})],
  [sp(2),S('◇ renovate','fg'),S('   renovate/go-1.25.3 → main   merged 41m ago · squash','dim')],
  [],
 ];
}
const JOBG={ok:S('✓','ok'),fail:S('✗','err'),run:S('○','warn'),wait:S('·','dim')};
function jobRow(j,o={}){
 const W=120; let l=[gut(o),JOBG[j[0]],sp(2),...fit([S(j[1],'fg',{b:o.sel})],29),sp(1),...fit([S(j[2],'dim')],11),sp(1),...fit(M(j[3]),W-2-1-2-30-12-9),S(j[4].padStart(8),'dim'),sp(1)];
 l=fit(l,W,false); return o.sel?withBg(l,'sel'):l;
}
function runHead(g,name,what,right){return lr([sp(2),g,sp(1),S(name,'fg',{b:1}),S('   '+what,'dim')],[S(right,'dim'),sp(1)],120);}
function pipelineScreen(){
 const W=120,L=prHead('pipeline');
 L.push(runHead(JOBG.run,'after merge','main @ 3f2a91c   push','2 of 4 passed   running 2m 41s'));
 [['ok','lint','prepare','{d|passed}','38s'],['ok','go mod verify','prepare','{d|passed}','21s'],
  ['run','build','build','{y|running}','1m 40s'],['wait','publish-image','deploy','{d|queued}','']]
  .forEach(j=>L.push(jobRow(j,{sel:j[1]==='build'})));
 L.push([]);
 L.push(runHead(JOBG.ok,'before merge','9c01e7a   merge request','4 of 4 passed   3m 05s'));
 L.push(M('      {d|zo to show its 4 jobs}'));
 while(L.length<38) L.push([]);
 L.push(status(W,{mode:'NORMAL',section:'#812',pos:'job 3/4'}));
 L.push(M('{d|Enter open job   zo zc show / fold a run   o open on gitlab.com   r refresh   q back to #812}'));
 return L.map(l=>fit(l,W,false));
}
// A job: its steps on the left (GitHub's steps, GitLab's log sections), its
// log on the right, following the tail while it runs.
function jobScreen(){
 const W=120,L=prHead('build');
 L.push(lr([sp(2),S('○','warn'),sp(1),S('build','fg',{b:1}),S('   build stage · after merge · main @ 3f2a91c','dim')],[S('running 1m 40s','dim'),sp(1)],W));
 L.push([]);
 const LW=40,RW=W-2-LW-3-1;
 L.push([sp(2),...fit(lr(M('{B|steps}'),M('{d|4}'),LW),LW),S(' │ ','faint'),...fit(lr(M('{aB|log}'),M('{d|14 lines   }{a|following ↓}'),RW),RW)]);
 const st=(g,n,t,sel)=>[sel?S('▌ ','acc'):sp(2),...lr(M(g+' '+n),M('{d| '+t+'}'),LW-2)];
 const left=[
  st('{g|✓}','Preparing the "docker" executor','4s'),
  st('{g|✓}','Preparing environment','1s'),
  st('{g|✓}','Getting source from Git','3s'),
  st('{y|○}','Executing "step_script" stage of the job script','1m 32s',true),
 ];
 const right=[
  M('{aB|Preparing the "docker" executor}'),
  M('Using docker image golang:1.25.3 ...'),
  M('{aB|Preparing environment}'),
  M('Running on runner-abbey via abbey...'),
  M('{aB|Getting source from Git}'),
  M('Fetching changes with git depth set to 20...'),
  M('Checking out 3f2a91c4 as detached HEAD (ref is main)...'),
  M('{aB|Executing "step_script" stage of the job script}'),
  M('{gB|$ go version}'),
  M('go version go1.25.3 linux/amd64'),
  M('{gB|$ make build}'),
  M('go build -trimpath -ldflags "-s -w" -o out/gitlab-runner .'),
  M('{y|warning: GOFLAGS is set; -mod=mod overrides the vendor directory}'),
  M('go: downloading golang.org/x/sys v0.37.0'),
 ];
 const bodyH=40-L.length-2;
 for(let i=0;i<bodyH;i++) L.push([sp(2),...fit(left[i]||[],LW),S(' │ ','faint'),...fit(right[i]||[],RW),sp(1)]);
 L.push(status(W,{mode:'NORMAL',section:'#812',pos:'log 14/14'}));
 L.push(M('{d|Enter jump to step   G follow   / search   e next error   o open on gitlab.com   q back to the pipeline}'));
 return L.map(l=>fit(l,W,false));
}

// ---------- row alternatives
function rowB(p,w,o={}){
 const sel=o.sel,g=sel?S('▌ ','acc'):sp(2);
 const a=lr([g,stateG(p),sp(1),p.armed?S('◆','acc'):sp(1),sp(2),S(p.t,p.draft?'dim':'fg',{b:sel})],[S(p.age,'dim'),sp(1)],w);
 const b=[g,sp(6),S(p.f+'  '+p.r+' #'+p.n,'dim'),sp(4),isAg(p)?S('◇ ','fg'):sp(0),S(p.a,'dim'),sp(4)];
 if(p.rv[1]) b.push(revS(p),S(' reviews','dim'),sp(4)); b.push(S('+'+p.ad+' −'+p.de,'dim'));
 let x=fit(a,w,false),y=fit(b,w,false); return sel?[withBg(x,'sel'),withBg(y,'sel')]:[x,y];
}
function sig(p){const ci={ok:S('✓','ok'),fail:S('✗','err'),run:S('○','warn'),none:S('◌','dim')}[p.ci];
 const rv=p.rv[1]?(p.rv[0]>=p.rv[1]?S('✓','ok'):S('○','warn')):S('–','faint');
 const mg=p.conflict?S('≠','err'):p.draft?S('◌','dim'):S('✓','ok'); return [ci,sp(2),rv,sp(2),mg];}
function rowC(p,w,o={}){const sel=o.sel;
 let l=lr([sel?S('▌ ','acc'):sp(2),sp(2),...sig(p),sp(3),p.armed?S('◆','acc'):sp(1),sp(2),S(('#'+p.n).padStart(5),sel?'fg':'dim'),sp(2),S(p.t,p.draft?'dim':'fg',{b:sel})],
  [...fit([isAg(p)?S('◇ ','fg'):sp(2),S(p.a,'dim')],14),S(p.age.padStart(4),'dim'),sp(1)],w);
 return sel?withBg(fit(l,w,false),'sel'):fit(l,w,false);}
function alts(){
 const W=120;
 const a=[headA(W-1),...P.slice(0,12).map((p,i)=>rowA(p,W-1,{sel:i===0}))].map(l=>fit(l,W,false));
 const b=P.slice(0,6).flatMap((p,i)=>rowB(p,W,{sel:i===0}));
 const groups={}; P.slice(0,16).forEach(p=>{(groups[p.r]=groups[p.r]||[]).push(p);});
 const c=[[sp(4),S('ci rv mg','faint')]]; let first=true;
 for(const r of ['acme/api','platform/runner','acme/web']){const ps=groups[r]; c.push(lr([sp(4),S(r,'fg',{b:1})],[S(ps[0].f,'dim'),sp(1)],W));
  ps.forEach(p=>{c.push(rowC(p,W,{sel:first}));first=false;}); if(c.length<13) c.push([]);}
 return {a,b,c:c.slice(0,13).map(l=>fit(l,W,false))};
}

// ---------- finalize
function fin(lines,mode){
 const Pl=mode.c16?PAL.c16:PAL.tc, acc=mode.c16?Pl.acc:(mode.accent||Pl.acc);
 const col=t=>t==='acc'?acc:(Pl[t]||t);
 return lines.map(l=>{const segs=[];
  for(const s of l){ if(!s.t) continue; let t=s.t; if(mode.ascii) t=[...t].map(ch=>ASCII[ch]??ch).join('');
   let k=s.k,b=s.b; if(mode.c16&&k==='sel'){k=null;b=true;} if(mode.c16&&k==='surf') k=null;
   const fg=col(s.f),bg=k?col(k):'transparent',fw=b?'700':'400',td=s.u?'underline':'none';
   for(const p of (t.match(/[\x20-\x7e]+|[^\x20-\x7e]/gu)||[])){ const g=!/^[\x20-\x7e]+$/.test(p)&&!'│▌…─·−–'.includes(p);
    segs.push({t:p,fg,bg,fw,td,w:g?'0.6667em':[...p].length+'ch',fs:g?'0.9em':'1em',ta:g?'center':'left',z:g?'1':'auto'}); } }
  return {segs};});
}

function build(mode){
 const F=l=>fin(l,mode), ST={mode:'NORMAL',section:'Agent-opened',pos:'1/23'};
 const out={};
 out.sList=F(listScreen({W:120,H:40,tab:5,prs:ALL,sel:1,preview:preview4821(46),right:'is:open',st:{mode:'NORMAL',section:'Open',pos:'2/96'}}));
 const hl='ledger', fi=ALL.findIndex(p=>p.t.toLowerCase().includes(hl));
 out.sFilter=F(listScreen({W:120,H:40,tab:5,prs:ALL,sel:fi,hl,right:'is:open   sort: updated',st:{mode:'FILTER',section:'Open',pos:'1/5 matches'},cmd:lr(M('/ledger{C| }'),M('{d|n N jump   Enter hide non-matches   Esc clear}'),120)}));
 const l80={W:80,H:24,tab:5,short:true,prs:ALL,sel:1,st:{mode:'NORMAL',section:'Open',pos:'2/96',narrow:true}};
 out.s80=F(listScreen(l80));
 out.s80ascii=fin(listScreen(l80),{ascii:true,c16:true});
 out.sDetail=F(detailScreen());
 out.sHelp=F(helpScreen());
 out.sArmed=F(armedScreen());
 out.sDeploys=F(deploysScreen());
 out.sMerged=F(mergedScreen());
 out.sPipeline=F(pipelineScreen());
 out.sJob=F(jobScreen());
 const al=alts(); out.altA=F(al.a); out.altB=F(al.b); out.altC=F(al.c);

 const R4=[P[0],P[1],P[2],P[3]];
 out.mergeFlow=[
  {k:'M · ready PR',cap:'Confirm, with method',note:'Line turns into a one-key prompt. y or Enter uses the repo default; m or r merges immediately with that method instead.',
   lines:strip({prs:R4,sel:1,st:{mode:'CONFIRM',modeK:'warn'},cmd:M('{f|Merge }{B|#812}{f| platform/runner → main with }{yB|squash}{f|?}   {d|y/Enter yes   m merge commit   r rebase   n/Esc cancel}')})},
  {k:'y',cap:'Merging',note:'Spinner replaces the state glyph on the row; the line reports which forge is being called.',
   lines:strip({prs:R4,sel:1,spin:812,cmd:M('{a|⠹} {f|Merging #812 on gitlab.com…}')})},
  {k:'done',cap:'Merged',note:'Row leaves the section; the cursor stays at the same index. Message clears on the next key.',
   lines:strip({prs:[P[0],P[2],P[3],P[4]],sel:1,st:{pos:'2/22'},cmd:M('{g|✓} {f|Merged #812 into main}{d|   squash · source branch deleted}')})},
  {k:'M · blocked PR',cap:'Refused, with the reason',note:'No prompt when the forge would reject the merge. The error names what blocks and suggests the next key.',
   lines:strip({prs:R4,sel:2,cmd:M('{r|E  #2290 can\'t merge: e2e/chromium failing · 1 review required}{d|      d diff   o open in browser}')})},
  {k:'M · not ready yet',cap:'Offer to arm instead',note:'CI still running or reviews pending: M offers auto-merge so the keystroke is not wasted.',
   lines:strip({prs:R4,sel:3,st:{mode:'CONFIRM',modeK:'warn'},cmd:M('{f|#377 isn\'t ready: CI 4/9 running, 1 review required. Arm auto-merge instead?}   {d|y/A arm   n/Esc cancel}')})},
  {k:'A · method choice',cap:'Arm, picking a method',note:'Only shown when the repo allows several methods and none is configured. One key picks; Enter takes the highlighted default.',
   lines:strip({prs:R4,sel:3,st:{mode:'CONFIRM',modeK:'warn'},cmd:M('{f|Arm auto-merge on #377 with}   {aB|s squash}   {f|m}{d| merge}   {f|r}{d| rebase}      {d|Enter squash   Esc cancel}')})},
  {k:'armed',cap:'Armed, waiting',note:'◆ appears on the row. The message lists exactly what the merge is waiting on; the PR also joins the Auto-merge armed section.',
   lines:strip({prs:[P[0],P[1],P[2],A(377,{armed:1})],sel:3,st:{armed:8},cmd:M('{a|◆} {f|Armed #377}{d|   squash · waiting on CI (4/9) and 1 review from @acme/infra}')})},
  {k:'A · on armed PR',cap:'Disarm',note:'A toggles. Disarming is not destructive, so it does not ask.',
   lines:strip({prs:R4,sel:3,cmd:M('{f|Auto-merge off for #377}{d|   A to re-arm}')})},
 ];
 const MK=[812,377,143,1203,375], R13=P.slice(0,13);
 out.batch=[
  {k:'Space × 6, then A',cap:'Batch arm',note:'Space marks rows (• in the gutter); V marks a range. With marks present, every action and command targets the marks.',
   lines:strip({prs:R13,sel:12,marks:MK,st:{mode:'CONFIRM',modeK:'warn',pos:'13/23',extra:[S('5 marked','acc')]},cmd:M('{f|Arm auto-merge on }{B|5 PRs}{f| with }{yB|squash}{f|?}   {d|y/Enter yes   m merge   r rebase   n/Esc cancel}')})},
  {k:'y',cap:'Result',note:'Rows that are already green merge right away. Ineligible rows are skipped and named, never silently dropped.',
   lines:strip({prs:R13.map(p=>MK.includes(p.n)&&!p.draft?{...p,armed:1}:p),sel:12,spin:812,st:{pos:'13/23',armed:11},cmd:M('{a|◆} {f|Armed 4 PRs}{d|   #812 merging now (already green) · skipped #1203: draft}')})},
 ];
 const R5=P.slice(0,5), CM={mode:'COMMAND'};
 out.cmd=[
  {k:'idle',cap:'Idle',note:'Empty. The last row belongs to the command line at all times, so nothing shifts when it opens.',lines:strip({prs:R5,sel:1})},
  {k:'typing',cap:'Typing',note:'Target resolves live on the right. Without a target the command acts on the selected row.',
   lines:strip({prs:R5,sel:1,st:CM,cmd:lr(M(':comment #812 rebased on main, merging after the 2.4 cut{C| }'),M('{d|→ platform/runner #812}  '),120)})},
  {k:':a then Tab',cap:'Completing a command',note:'Wildmenu opens upward over the list. Tab / S-Tab cycle and fill the line; typing narrows.',
   lines:strip({prs:R5,sel:1,st:CM,menuW:56,menu:[withBg(fit(M(' {B|approve}       {d|approve the PR}'),56),'sel'),M(' automerge     {d|arm or disarm auto-merge}')],cmd:M(':approve{C| }')})},
  {k:':merge #48 then Tab',cap:'Completing a target',note:'Targets complete from the current section with state glyph and title, so you pick the right #.',
   lines:strip({prs:R5,sel:1,st:CM,menu:[menuPR(P[0],true,84),menuPR(P[5],false,84),menuPR(P[10],false,84),menuPR(P[16],false,84)],menuW:84,cmd:M(':merge #4821{C| }')})},
  {k:'confirming',cap:'Confirming',note:'Mode block turns yellow. One key answers; anything other than y / Enter / a method key cancels.',
   lines:strip({prs:R5,sel:0,st:{mode:'CONFIRM',modeK:'warn'},cmd:M('{f|Close }{B|#4821}{f| without merging?}   {d|y/Enter yes   n/Esc cancel}')})},
  {k:'error',cap:'Error',note:'Red E prefix, the reason, then the most useful next key. Stays until the next keypress.',
   lines:strip({prs:R5,sel:2,st:CM,cmd:M('{r|E  :approve #2290: you are the author of this PR\'s last commit (repo requires independent review)}')})},
  {k:'unknown',cap:'Typo',note:'Unknown commands suggest the nearest match; Tab accepts the suggestion.',
   lines:strip({prs:R5,sel:1,st:CM,cmd:M('{r|E  not a command: :mrege}{d|   did you mean :merge?  Tab to fix}')})},
  {k:'success',cap:'Success',note:'Green check, past tense, details dim.',
   lines:strip({prs:R5,sel:1,cmd:M('{g|✓} {f|Approved #812}{d|   platform/runner · 2 of 1 required (auto-merge not armed)}')})},
  {k:'↑ in :',cap:'History',note:'↑ / ↓ walk history filtered by what is already typed. Persisted per user across sessions.',
   lines:strip({prs:R5,sel:1,st:CM,cmd:lr(M(':comment #812 lgtm, merging after the release{C| }'),M('{d|history 3/14}  '),120)})},
  {k:'background',cap:'Sync error',note:'Errors not caused by a command go to the status line, right side, and stay until resolved. :messages lists them.',
   lines:strip({prs:R5,sel:1,st:{err:'gitlab.com 502 · retrying in 8s'}})},
 ];
 for(const k of ['mergeFlow','batch','cmd']) out[k]=out[k].map(x=>{const L=F(x.lines);return {...x,lines:L,segs:L.flatMap(l=>l.segs)};});
 return out;
}
window.TUI={build};
})();

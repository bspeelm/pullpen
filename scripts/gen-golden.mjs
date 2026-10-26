// Renders every screen in design/tui.js to tests/golden/ (ADR-002).
//
//   node scripts/gen-golden.mjs
//
// tui.js only exposes build(), whose output is shaped for the browser, so this
// undoes what its fin() did: blank rows come back with no segments and are
// padded to the screen width, glyphs come back as their own segments and are
// merged into runs of one style, and colours come back as hex and are mapped
// to the names the Rust renderer uses.
//
// The hex-to-name maps are read out of tui.js's own PAL rather than written
// here, so this file holds no colour values (ADR-003) and cannot drift from
// the design.

import { readFileSync, writeFileSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const src = readFileSync(join(root, 'design/tui.js'), 'utf8');
const out = join(root, 'tests/golden');

const sandbox = { window: {} };
vm.runInNewContext(src, sandbox);
const TUI = sandbox.window.TUI;

function palette(tier) {
  const m = src.match(new RegExp(`\\b${tier}:\\{([^}]*)\\}`));
  if (!m) throw new Error(`PAL.${tier} not found in tui.js`);
  return Object.fromEntries([...m[1].matchAll(/(\w+):'([^']*)'/g)].map(x => [x[1], x[2]]));
}

// Token names follow the Rust Tok enum, lower-cased.
const NAME = { bg: 'bg', surf: 'surf', sel: 'sel', fg: 'fg', dim: 'dim', faint: 'faint', acc: 'acc', ok: 'ok', warn: 'warn', err: 'err' };

// Truecolor: every value is distinct, so hex maps back to its token exactly.
const tc = palette('tc');
const tcName = {};
for (const [k, v] of Object.entries(tc)) {
  if (tcName[v]) throw new Error(`PAL.tc reuses ${v}; tokens are no longer recoverable`);
  tcName[v] = NAME[k];
}

// 16 colours: dim and faint share a value, so the token cannot be recovered.
// The golden records the colour the terminal is asked for instead, in the
// names the design's 16-colour column uses (spec §10).
const C16 = { bg: 'black', fg: 'default', dim: 'brightblack', faint: 'brightblack', acc: 'cyan', ok: 'green', warn: 'yellow', err: 'red' };
const c16 = palette('c16');
const c16Name = {};
for (const [k, v] of Object.entries(c16)) {
  if (v === 'transparent') continue;
  const n = C16[k];
  if (c16Name[v] && c16Name[v] !== n) throw new Error(`PAL.c16 ${v} is both ${c16Name[v]} and ${n}`);
  c16Name[v] = n;
}

function lines(screen, names, width) {
  const plain = [], styled = [];
  for (const l of screen) {
    const runs = [];
    for (const s of l.segs) {
      const st = {
        fg: names[s.fg] ?? fail(`unmapped fg ${s.fg}`),
        bg: s.bg === 'transparent' ? null : (names[s.bg] ?? fail(`unmapped bg ${s.bg}`)),
        b: s.fw === '700',
        u: s.td === 'underline',
      };
      const last = runs[runs.length - 1];
      if (last && last.fg === st.fg && last.bg === st.bg && last.b === st.b && last.u === st.u) last.t += s.t;
      else runs.push({ ...st, t: s.t });
    }
    const n = runs.reduce((a, r) => a + [...r.t].length, 0);
    if (n > width) fail(`row is ${n} cells, wider than ${width}`);
    if (n < width) {
      const last = runs[runs.length - 1];
      if (last && last.fg === 'fg' && !last.bg && !last.b && !last.u) last.t += ' '.repeat(width - n);
      else runs.push({ fg: names === tcName ? 'fg' : 'default', bg: null, b: false, u: false, t: ' '.repeat(width - n) });
    }
    plain.push(runs.map(r => r.t).join(''));
    styled.push(runs.map(r => `{${tag(r)}}${r.t}`).join(''));
  }
  return { plain: plain.join('\n') + '\n', styled: styled.join('\n') + '\n' };
}

const tag = r => [r.fg, r.bg && `bg=${r.bg}`, r.b && 'b', r.u && 'u'].filter(Boolean).join(' ');
function fail(msg) { throw new Error(msg); }

mkdirSync(out, { recursive: true });
for (const f of readdirSync(out)) if (f.endsWith('.txt')) rmSync(join(out, f));

function write(name, screen, names, width) {
  const { plain, styled } = lines(screen, names, width);
  writeFileSync(join(out, `${name}.txt`), plain);
  writeFileSync(join(out, `${name}.ansi.txt`), styled);
}

const full = TUI.build({});
const W = { sList: 120, sFilter: 120, s80: 80, sDetail: 120, sHelp: 120, sArmed: 120, sMerged: 120 };
for (const [k, w] of Object.entries(W)) write(k, full[k], tcName, w);
write('s80ascii', full.s80ascii, c16Name, 80);
for (const group of ['mergeFlow', 'batch', 'cmd']) {
  full[group].forEach((step, i) => write(`${group}-${i}`, step.lines, tcName, 120));
}
console.log(`wrote ${readdirSync(out).length} files to tests/golden/`);

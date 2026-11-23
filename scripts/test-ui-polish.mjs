 
 
import assert from 'node:assert/strict';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const read = path => readFileSync(resolve(root, path), 'utf8');
const html = read('static/index.html');
const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
assert.equal(new Set(ids).size, ids.length, 'Duplicate HTML IDs break event routing');
for (const match of html.matchAll(/(?:src|href)="(\/static\/[^"#]+)"/g)) {
  assert(existsSync(resolve(root, match[1].slice(1))), `Missing asset ${match[1]}`);
}
for (const match of html.matchAll(/<use\b[^>]*href="#([^"]+)"/g)) {
  assert(ids.includes(match[1]), `Missing SVG symbol #${match[1]}`);
}
assert(!/fonts\.googleapis|fonts\.gstatic/.test(html), 'Fonts must work offline');
for (const match of read('static/fonts/fonts.css').matchAll(/url\(\.\/([^\)]+)\)/g)) {
  assert(existsSync(resolve(root, 'static/fonts', match[1])), `Missing bundled font ${match[1]}`);
}
const base = JSON.parse(read('src-tauri/tauri.conf.json'));
assert.equal(base.app.windows[0].decorations, false);
for (const name of ['', '.linux', '.macos']) {
  const config = JSON.parse(read(`src-tauri/tauri${name}.conf.json`));
  const main = config.app.windows.find(item => item.label === 'main');
  assert.equal(main.devtools, false);
  assert.equal(main.zoomHotkeysEnabled, false);
  assert.equal(main.allowLinkPreview, false);
}
if (existsSync(resolve(root, 'dist/index.html'))) {
  const built = read('dist/index.html');
  assert.equal([...built.matchAll(/src="\/static\/desktop\.js"/g)].length, 1);
  for (const match of built.matchAll(/(?:src|href)="(\/static\/[^"#]+)"/g)) {
    assert(existsSync(resolve(root, 'dist', match[1].slice(1))), `Missing built asset ${match[1]}`);
  }
  for (const name of readdirSync(resolve(root, 'static'))) {
    if (name.endsWith('.js') || name.endsWith('.css')) assert.equal(read(`dist/static/${name}`), read(`static/${name}`), `Rebuild stale dist/static/${name}`);
  }
}
console.log('PASS: local fonts/assets, unique IDs, SVG symbols, desktop configurations and current dist.');

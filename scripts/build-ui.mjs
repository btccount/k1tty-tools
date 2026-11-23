import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(root, 'dist');
await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
await cp(resolve(root, 'static'), resolve(out, 'static'), { recursive: true });
let html = await readFile(resolve(root, 'static/index.html'), 'utf8');
html = html.replace(/<link rel="preconnect" href="https:\/\/fonts\.(?:googleapis|gstatic)\.com"[^>]*>\n?/g, '');
html = html.replace(/<link href="https:\/\/fonts\.googleapis\.com\/css2[^>]*>/, '<link rel="stylesheet" href="/static/fonts/fonts.css">');
html = html.replace('<script src="/static/winter.js"></script>', '<script src="/static/desktop.js"></script>\n<script src="/static/winter.js"></script>');
await writeFile(resolve(out, 'index.html'), html);
await rm(resolve(out, 'static/index.html'));
console.log('Original K1tty UI copied to dist with native IPC and bundled fonts.');

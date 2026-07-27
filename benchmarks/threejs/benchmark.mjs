import { createServer } from 'node:http';
import { readFile, stat, writeFile, mkdir } from 'node:fs/promises';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const here = dirname(fileURLToPath(import.meta.url));
const workspace = resolve(here, '..', '..');
const model = resolve(process.argv[2] ?? join(workspace, 'target/bench/models/DamagedHelmet.glb'));
const iterations = Math.max(3, Number(process.argv[3] ?? 11));
const backend = process.argv[4] ?? 'webgl2';
if (!['webgl2', 'webgpu'].includes(backend)) throw new Error(`unsupported backend: ${backend}`);
const output = resolve(process.argv[5] ?? join(workspace, `target/bench/threejs/benchmark-${backend}.json`));
const chromeCandidates = process.platform === 'win32'
  ? ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe']
  : process.platform === 'darwin'
    ? ['/Applications/Google Chrome.app/Contents/MacOS/Google Chrome']
    : ['/usr/bin/google-chrome', '/usr/bin/chromium', '/usr/bin/chromium-browser'];
let executablePath;
for (const candidate of chromeCandidates) {
  try { await stat(candidate); executablePath = candidate; break; } catch {}
}
if (!executablePath) throw new Error('Chrome/Chromium executable was not found');

const mime = new Map([['.html','text/html'],['.js','text/javascript'],['.mjs','text/javascript'],['.glb','model/gltf-binary']]);
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://127.0.0.1');
    let path;
    if (url.pathname === '/') path = join(here, 'benchmark.html');
    else if (url.pathname === '/model.glb') path = model;
    else if (url.pathname.startsWith('/node_modules/')) path = join(here, url.pathname);
    else throw new Error('not found');
    const bytes = await readFile(path);
    response.writeHead(200, {'content-type': mime.get(extname(path)) ?? 'application/octet-stream', 'cache-control':'no-store'});
    response.end(bytes);
  } catch (error) {
    response.writeHead(404); response.end(String(error));
  }
});
await new Promise(resolveReady => server.listen(0, '127.0.0.1', resolveReady));
const { port } = server.address();
const browser = await chromium.launch({
  executablePath,
  headless: true,
  args: [
    '--enable-gpu',
    '--ignore-gpu-blocklist',
    '--use-angle=d3d11',
    '--force_high_performance_gpu',
    '--enable-unsafe-webgpu'
  ]
});
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 1024 }, deviceScaleFactor: 1 });
  await page.goto(`http://127.0.0.1:${port}/?backend=${backend}`, { waitUntil: 'load' });
  await page.waitForFunction(() => typeof window.initialize === 'function');
  const setup = await page.evaluate(() => window.initialize('/model.glb'));
  const results = await page.evaluate(count => window.runBenchmark(count), iterations);
  const report = {
    generated_at: new Date().toISOString(),
    classification: `persistent headless Chrome; Three.js requested ${backend}; model and GPU resources resident; browser timings include GPU finish and canvas PNG encode`,
    three_version: '0.185.1',
    playwright_core_version: '1.62.0',
    chrome: await browser.version(),
    executable: executablePath,
    model,
    iterations,
    requested_backend: backend,
    setup,
    results
  };
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser.close();
  await new Promise(resolveClose => server.close(resolveClose));
}

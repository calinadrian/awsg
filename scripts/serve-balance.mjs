import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../web');
const files = new Map([
  ['/balance.html', ['balance.html', 'text/html; charset=utf-8']],
  ['/balance.js', ['balance.js', 'text/javascript; charset=utf-8']],
  ['/balance-data.js', ['balance-data.js', 'text/javascript; charset=utf-8']],
  ['/zones.html', ['zones.html', 'text/html; charset=utf-8']],
  ['/zones.js', ['zones.js', 'text/javascript; charset=utf-8']],
  ['/zones-data.js', ['zones-data.js', 'text/javascript; charset=utf-8']],
]);

const server = createServer(async (request, response) => {
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    response.writeHead(405, { Allow: 'GET, HEAD' }).end();
    return;
  }
  const pathname = new URL(request.url ?? '/', 'http://localhost').pathname;
  if (pathname === '/') {
    response.writeHead(302, { Location: '/balance.html' }).end();
    return;
  }
  const entry = files.get(pathname);
  if (!entry) {
    response.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' }).end('Not found');
    return;
  }
  try {
    const body = await readFile(path.join(root, entry[0]));
    response.writeHead(200, { 'Content-Type': entry[1], 'Content-Length': body.length, 'Cache-Control': 'no-store' });
    response.end(request.method === 'HEAD' ? undefined : body);
  } catch {
    response.writeHead(500, { 'Content-Type': 'text/plain; charset=utf-8' }).end('Could not read simulator files');
  }
});

server.listen(8091, '127.0.0.1', () => {
  process.stdout.write('Azeroth Survivors tools: http://127.0.0.1:8091/balance.html and http://127.0.0.1:8091/zones.html\nPress Ctrl+C to stop.\n');
});

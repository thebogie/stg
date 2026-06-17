#!/usr/bin/env node
/**
 * Run STG smoke Playwright spec against STG_BASE_URL.
 * Usage: node smoke-stg.mjs [baseUrl]
 */
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const baseUrl = process.argv[2] || process.env.STG_BASE_URL || 'http://localhost:8080';
const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const spec = join(root, 'testing', 'e2e', 'smoke.stg.spec.ts');

const child = spawn(
  'npx',
  ['playwright', 'test', spec, '--config', join(root, 'testing', 'playwright.config.ts')],
  {
    cwd: root,
    env: { ...process.env, STG_BASE_URL: baseUrl, CI: process.env.CI || '1' },
    stdio: ['ignore', 'pipe', 'pipe'],
  },
);

let stdout = '';
let stderr = '';
child.stdout.on('data', (c) => {
  stdout += c.toString();
});
child.stderr.on('data', (c) => {
  stderr += c.toString();
});

child.on('close', (code) => {
  if (code === 0) {
    console.log(stdout.trim() || `STG smoke passed (${baseUrl})`);
    process.exit(0);
  }
  console.error(stderr.trim() || stdout.trim() || `smoke failed exit ${code}`);
  process.exit(code ?? 1);
});

#!/usr/bin/env node
/**
 * STG Playwright worker — dequeue browser jobs from Redis and run site-specific scripts.
 *
 * Env:
 *   REDIS_URL              default redis://redis:6379/
 *   PLAYWRIGHT_JOBS_DIR    default /jobs
 *   BGG_SCRIPT_PATH        default /app/tools/bgg-marketplace/fill-listing.mjs
 *   BGG_HEADLESS           default 1
 *   BGG_AUTO_SUBMIT        default 0
 *   PLAYWRIGHT_JOB_TTL_SECONDS    default 900
 *   PLAYWRIGHT_STATUS_TTL_SECONDS default 3600
 */

import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import Redis from 'ioredis';

const QUEUE_KEY = 'playwright:queue';
const JOB_PREFIX = 'playwright:job:';
const STATUS_PREFIX = 'playwright:status:';

const REDIS_URL = process.env.REDIS_URL || 'redis://redis:6379/';
const JOBS_DIR = process.env.PLAYWRIGHT_JOBS_DIR || '/jobs';
const BGG_SCRIPT = process.env.BGG_SCRIPT_PATH || '/app/tools/bgg-marketplace/fill-listing.mjs';
const SMOKE_SCRIPT = process.env.SMOKE_SCRIPT_PATH || '/app/tools/playwright-worker/smoke-stg.mjs';
const JOB_TTL = Number(process.env.PLAYWRIGHT_JOB_TTL_SECONDS || 900);
const STATUS_TTL = Number(process.env.PLAYWRIGHT_STATUS_TTL_SECONDS || 3600);

const JOB_TYPE_BGG_FILL = 'bgg.geekmarket.fill';
const JOB_TYPE_SMOKE_STG = 'smoke.stg';
const STATUS_QUEUED = 'queued';
const STATUS_RUNNING = 'running';
const STATUS_COMPLETED = 'completed';
const STATUS_FAILED = 'failed';

const redis = new Redis(REDIS_URL, { maxRetriesPerRequest: null });

function log(...args) {
  console.log('[playwright-worker]', ...args);
}

async function setStatus(job, patch) {
  const key = `${STATUS_PREFIX}${job.job_id}`;
  const raw = await redis.get(key);
  const base = raw
    ? JSON.parse(raw)
    : {
        job_id: job.job_id,
        listing_id: job.listing_id,
        player_id: job.player_id,
        job_type: job.job_type,
        status: STATUS_QUEUED,
      };
  const next = { ...base, ...patch };
  await redis.setex(key, STATUS_TTL, JSON.stringify(next));
}

async function runBggFill(job) {
  const workDir = join(JOBS_DIR, job.job_id);
  mkdirSync(workDir, { recursive: true });
  const jsonPath = join(workDir, 'listing.json');
  writeFileSync(jsonPath, JSON.stringify(job.payload, null, 2), 'utf8');

  const headless = process.env.BGG_HEADLESS !== '0' ? '1' : '0';
  const autoSubmit = process.env.BGG_AUTO_SUBMIT === '1' ? '1' : '0';

  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [BGG_SCRIPT, jsonPath], {
      env: {
        ...process.env,
        BGG_USERNAME: job.secrets.bgg_username,
        BGG_PASSWORD: job.secrets.bgg_password,
        BGG_HEADLESS: headless,
        BGG_AUTO_SUBMIT: autoSubmit,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('error', reject);
    child.on('close', (code) => {
      try {
        writeFileSync(join(workDir, 'stdout.log'), stdout, 'utf8');
        writeFileSync(join(workDir, 'stderr.log'), stderr, 'utf8');
      } catch {
        /* best effort */
      }
      if (code === 0) {
        const message =
          stdout.trim() ||
          'BGG form filled — review listing on BoardGameGeek and submit.';
        resolve({ message });
      } else {
        const err = stderr.trim() || stdout.trim() || `exit code ${code}`;
        reject(new Error(err));
      }
    });
  });
}

async function runSmokeStg(job) {
  const baseUrl = job.payload?.base_url || process.env.STG_BASE_URL || 'http://localhost:8080';
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [SMOKE_SCRIPT, baseUrl], {
      env: { ...process.env, STG_BASE_URL: baseUrl },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) {
        resolve({ message: stdout.trim() || `STG smoke passed (${baseUrl})` });
      } else {
        reject(new Error(stderr.trim() || stdout.trim() || `exit code ${code}`));
      }
    });
  });
}

async function dispatch(job) {
  switch (job.job_type) {
    case JOB_TYPE_BGG_FILL:
      return runBggFill(job);
    case JOB_TYPE_SMOKE_STG:
      return runSmokeStg(job);
    default:
      throw new Error(`unknown job type: ${job.job_type}`);
  }
}

async function processJob(jobId) {
  const jobKey = `${JOB_PREFIX}${jobId}`;
  const raw = await redis.get(jobKey);
  if (!raw) {
    log(`job ${jobId} missing or expired — skipping`);
    return;
  }

  const job = JSON.parse(raw);
  log(`running ${job.job_type} job ${jobId} for ${job.listing_id}`);

  await setStatus(job, {
    status: STATUS_RUNNING,
    message: 'Running browser automation…',
    error: null,
  });

  try {
    const { message } = await dispatch(job);
    await setStatus(job, {
      status: STATUS_COMPLETED,
      message,
      error: null,
    });
    log(`completed job ${jobId}`);
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err);
    await setStatus(job, {
      status: STATUS_FAILED,
      message: null,
      error,
    });
    log(`failed job ${jobId}: ${error}`);
  } finally {
    await redis.del(jobKey);
  }
}

async function loop() {
  log(`listening on ${REDIS_URL} queue=${QUEUE_KEY}`);
  while (true) {
    const result = await redis.brpop(QUEUE_KEY, 0);
    if (!result) continue;
    const jobId = result[1];
    await processJob(jobId);
  }
}

loop().catch((err) => {
  console.error('[playwright-worker] fatal:', err);
  process.exit(1);
});

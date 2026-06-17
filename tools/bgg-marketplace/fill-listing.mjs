#!/usr/bin/env node
/**
 * Log into BGG and fill GeekMarket listing form from STG export JSON.
 *
 * Usage: node fill-listing.mjs path/to/listing.json
 *
 * Env: BGG_USERNAME, BGG_PASSWORD (required), BGG_HEADLESS, BGG_AUTO_SUBMIT
 */

import { readFileSync } from 'node:fs';
import { chromium } from 'playwright';

const listingPath = process.argv[2];
if (!listingPath) {
  console.error('Usage: node fill-listing.mjs <listing.json>');
  process.exit(1);
}

const username = process.env.BGG_USERNAME;
const password = process.env.BGG_PASSWORD;
if (!username || !password) {
  console.error('BGG_USERNAME and BGG_PASSWORD env vars are required');
  process.exit(1);
}

const payload = JSON.parse(readFileSync(listingPath, 'utf8'));
const headless = process.env.BGG_HEADLESS !== '0';
const autoSubmit = process.env.BGG_AUTO_SUBMIT === '1';

const CONDITION_LABELS = {
  new: 'New',
  like_new: 'Like New',
  very_good: 'Very Good',
  good: 'Good',
  acceptable: 'Acceptable',
};

const browser = await chromium.launch({ headless });
const page = await browser.newPage();

async function tryFill(selectors, value) {
  for (const sel of selectors) {
    const el = page.locator(sel).first();
    if ((await el.count()) > 0) {
      await el.fill(String(value));
      return true;
    }
  }
  return false;
}

async function trySelect(selectors, label) {
  for (const sel of selectors) {
    const el = page.locator(sel).first();
    if ((await el.count()) > 0) {
      try {
        await el.selectOption({ label });
        return true;
      } catch {
        try {
          await el.selectOption(label);
          return true;
        } catch {
          /* continue */
        }
      }
    }
  }
  return false;
}

async function tryCheck(selectors) {
  for (const sel of selectors) {
    const el = page.locator(sel).first();
    if ((await el.count()) > 0) {
      await el.check();
      return true;
    }
  }
  return false;
}

// --- Login ---
await page.goto('https://boardgamegeek.com/login', { waitUntil: 'domcontentloaded', timeout: 60_000 });
await page.waitForTimeout(1500);

await tryFill(
  ['input[name="username"]', 'input#username', 'input[type="text"]'],
  username,
);
await tryFill(
  ['input[name="password"]', 'input#password', 'input[type="password"]'],
  password,
);

const loginBtn = page
  .locator('button[type="submit"], input[type="submit"], button:has-text("Sign In"), button:has-text("Log In")')
  .first();
if ((await loginBtn.count()) > 0) {
  await loginBtn.click();
  await page.waitForLoadState('domcontentloaded');
  await page.waitForTimeout(2000);
}

// --- GeekMarket create listing ---
const addUrls = [
  `https://boardgamegeek.com/geekmarket/add/${payload.bgg_id}`,
  'https://boardgamegeek.com/geekmarket/additem',
  'https://boardgamegeek.com/market',
];
let opened = false;
for (const url of addUrls) {
  try {
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30_000 });
    await page.waitForTimeout(1500);
    opened = true;
    break;
  } catch {
    /* try next */
  }
}
if (!opened) {
  await browser.close();
  console.error('Could not open GeekMarket listing page');
  process.exit(2);
}

const title = payload.title || payload.game_name || 'Board game';
const description = [
  payload.description,
  payload.condition_notes && `Condition: ${payload.condition_notes}`,
  payload.edition_notes && `Edition: ${payload.edition_notes}`,
  payload.missing_components?.length
    ? `Missing: ${payload.missing_components.join(', ')}`
    : null,
  payload.shipping_notes && `Shipping: ${payload.shipping_notes}`,
  payload.seller_notes,
]
  .filter(Boolean)
  .join('\n\n');

const price = (payload.price_cents ?? 0) / 100;
const conditionLabel = CONDITION_LABELS[payload.condition] || 'Very Good';

await tryFill(['input[name="title"]', 'input#title'], title);
await tryFill(['textarea[name="description"]', 'textarea#description', 'textarea'], description);
await tryFill(['input[name="price"]', 'input#price'], price.toFixed(2));
await trySelect(
  ['select[name="condition"]', 'select#condition'],
  conditionLabel,
);

if (payload.bgg_id) {
  await tryFill(
    ['input[name="objectid"]', 'input#objectid', 'input[name="gameid"]'],
    payload.bgg_id,
  );
}

if (payload.payment_paypal) {
  await tryCheck([
    'input[name="paypal"]',
    'input[value="paypal"]',
    'label:has-text("PayPal") input',
  ]);
}
if (payload.payment_other) {
  await tryCheck([
    'input[name="otherpayment"]',
    'label:has-text("Other") input',
  ]);
}

if (payload.item_location) {
  await tryFill(
    ['input[name="location"]', 'textarea[name="location"]'],
    payload.item_location,
  );
}
if (payload.ship_to) {
  await tryFill(
    ['input[name="willship"]', 'textarea[name="willship"]'],
    payload.ship_to,
  );
}

// Upload photos if file inputs exist
const photoPaths = payload.photo_paths || [];
if (photoPaths.length > 0) {
  const fileInput = page.locator('input[type="file"]').first();
  if ((await fileInput.count()) > 0) {
    try {
      await fileInput.setInputFiles(photoPaths);
    } catch (e) {
      console.warn(`Photo upload skipped: ${e.message}`);
    }
  }
}

const summary = [
  `Game: ${payload.game_name} (BGG ${payload.bgg_id})`,
  `Price: ${payload.currency || 'USD'} ${price.toFixed(2)}`,
  `Condition: ${conditionLabel}`,
  `Photos: ${photoPaths.length}`,
].join('\n');

if (autoSubmit) {
  const submit = page.locator('button[type="submit"], input[type="submit"]').first();
  if ((await submit.count()) > 0) {
    await submit.click();
    await page.waitForTimeout(3000);
    console.log(`Submitted listing.\n${summary}`);
  } else {
    console.log(`Form filled but submit button not found.\n${summary}`);
  }
} else {
  console.log(`Form filled — review and submit manually on BGG.\n${summary}`);
}

await browser.close();

# BGG Marketplace automation (Sell a Game)

Human-assisted Playwright script that fills a BGG sell listing from an STG export JSON.

## Setup

```bash
cd tools/bgg-marketplace
npm install
npx playwright install chromium
```

## Usage

1. Complete the STG `/sell` wizard through the automation handoff step.
2. Save the exported JSON as `listing.json`.
3. Run:

```bash
node fill-listing.mjs listing.json
```

4. Log in to BGG in the opened browser if needed.
5. Navigate to the marketplace sell form, press Enter in the terminal.
6. Review the pre-filled form and **submit manually on BGG**.

## Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `BGG_USER_DATA_DIR` | — | Persistent browser profile (keeps login) |
| `BGG_HEADLESS` | `0` | Set `1` for headless |
| `BGG_AUTO_SUBMIT` | `0` | Set `1` to attempt auto-submit (not recommended) |

See [docs/sell-game-guardrails.md](../../docs/sell-game-guardrails.md).

## Manual test checklist

- [ ] Export JSON from `/sell` after marketplace checkpoint
- [ ] Playwright opens BGG and accepts login
- [ ] Title, description, price fields populate (or operator fills gaps)
- [ ] Final listing submitted manually on BGG
- [ ] STG records automation result via API (optional)

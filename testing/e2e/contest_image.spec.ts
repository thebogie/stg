import { test, expect } from '@playwright/test';
import {
  e2eUserCreds,
  e2eApiBase,
  bearerAuth,
} from './helpers';

/**
 * Contest thumbnail API (upload → GET WebP → delete).
 * Runs in chromium-authenticated (see playwright.config.ts); uses gate-provisioned user.
 */
test.describe('Contest thumbnail image API', () => {
  test.skip(
    !e2eUserCreds(),
    'Set E2E_USER_EMAIL and E2E_USER_PASSWORD (see testing/e2e/global-setup.ts)',
  );

  // 1x1 PNG (red pixel)
  const MINIMAL_PNG = Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
    'base64',
  );

  test('upload returns WebP on GET and clears on DELETE', async ({ request }) => {
    const creds = e2eUserCreds()!;
    const apiBase = e2eApiBase();

    const loginRes = await request.post(`${apiBase}/api/players/login`, {
      headers: { 'Content-Type': 'application/json' },
      data: JSON.stringify({ email: creds.email, password: creds.password }),
    });
    const loginText = await loginRes.text();
    expect(
      loginRes.ok(),
      `POST /api/players/login: HTTP ${loginRes.status()} ${loginText}`,
    ).toBeTruthy();
    const login = JSON.parse(loginText) as { session_id: string };
    const auth = bearerAuth(login.session_id);

    const ts = Date.now();
    const placeId = `e2e_img_place_${ts}`;

    const venueRes = await request.post(`${apiBase}/api/venues`, {
      headers: auth,
      headers: { ...auth, 'Content-Type': 'application/json' },
      data: JSON.stringify({
        displayName: 'E2E Image Venue',
        formattedAddress: '1 Test St',
        place_id: placeId,
        lat: 40.7,
        lng: -74.0,
        timezone: 'America/New_York',
        source: 'database',
      }),
    });
    const venueText = await venueRes.text();
    expect(
      venueRes.ok(),
      `POST /api/venues: HTTP ${venueRes.status()} ${venueText}`,
    ).toBeTruthy();
    const venue = JSON.parse(venueText) as Record<string, unknown>;

    const gameRes = await request.post(`${apiBase}/api/games`, {
      headers: auth,
      headers: { ...auth, 'Content-Type': 'application/json' },
      data: JSON.stringify({
        name: `E2E Image Game ${ts}`,
        year_published: 2024,
        source: 'database',
      }),
    });
    const gameText = await gameRes.text();
    expect(gameRes.ok(), `POST /api/games: HTTP ${gameRes.status()} ${gameText}`).toBeTruthy();
    const game = JSON.parse(gameText) as Record<string, unknown>;

    const start = new Date().toISOString();
    const stop = new Date(Date.now() + 3600_000).toISOString();
    const contestRes = await request.post(`${apiBase}/api/contests`, {
      headers: auth,
      headers: { ...auth, 'Content-Type': 'application/json' },
      data: JSON.stringify({
        name: `E2E Image Contest ${ts}`,
        start,
        stop,
        venue: {
          id: venue.id ?? venue._id,
          displayName: venue.display_name ?? venue.displayName,
          formattedAddress: venue.formatted_address ?? venue.formattedAddress,
          place_id: venue.place_id ?? venue.placeId ?? placeId,
          lat: venue.lat,
          lng: venue.lng,
          timezone: venue.timezone,
          source: 'database',
        },
        games: [
          {
            id: game.id ?? game._id,
            name: game.name,
            year_published: game.year_published ?? game.yearPublished ?? 2024,
            source: 'database',
          },
        ],
        outcomes: [],
      }),
    });
    const contestText = await contestRes.text();
    expect(
      contestRes.ok(),
      `POST /api/contests: HTTP ${contestRes.status()} ${contestText}`,
    ).toBeTruthy();
    const contest = JSON.parse(contestText) as Record<string, unknown>;
    const contestId = String(contest.id ?? contest._id);
    const contestKey = contestId.includes('/') ? contestId.split('/').pop()! : contestId;

    const uploadRes = await request.put(`${apiBase}/api/contests/${contestKey}/image`, {
      headers: { ...auth, 'Content-Type': 'image/png' },
      data: MINIMAL_PNG,
    });
    const uploadText = await uploadRes.text();
    expect(
      uploadRes.ok(),
      `PUT /api/contests/${contestKey}/image: HTTP ${uploadRes.status()} ${uploadText}`,
    ).toBeTruthy();
    const uploaded = JSON.parse(uploadText) as Record<string, unknown>;
    expect(uploaded.has_image).toBe(true);
    expect(String(uploaded.image_url)).toContain('/image');

    const imageRes = await request.get(`${apiBase}/api/contests/${contestKey}/image`, {
      headers: auth,
    });
    expect(
      imageRes.ok(),
      `GET /api/contests/${contestKey}/image: HTTP ${imageRes.status()}`,
    ).toBeTruthy();
    expect(imageRes.headers()['content-type']).toContain('image/webp');
    const body = await imageRes.body();
    expect(body.length).toBeGreaterThan(12);
    expect(body.subarray(0, 4).toString()).toBe('RIFF');
    expect(body.subarray(8, 12).toString()).toBe('WEBP');

    const delRes = await request.delete(`${apiBase}/api/contests/${contestKey}/image`, {
      headers: auth,
    });
    expect(delRes.status()).toBe(204);

    const goneRes = await request.get(`${apiBase}/api/contests/${contestKey}/image`, {
      headers: auth,
    });
    expect(goneRes.status()).toBe(404);
  });
});

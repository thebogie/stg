# Structured logging and observability

STG uses **structured JSON logs** in production, **correlation IDs** on every HTTP request, and an optional **Loki + Grafana** stack for centralized search.

## Quick start

1. Start the main stack: `./scripts/start-back.sh`
2. Start observability: `./scripts/start-observability.sh`
3. Open Grafana: `http://localhost:3000` (default user `admin`, password from `GRAFANA_ADMIN_PASSWORD` in env)

## Environment variables

| Variable | Dev default | Prod default | Purpose |
|----------|---------------|--------------|---------|
| `RUST_LOG` | `debug` | `info` | Log filter (`backend=debug`, etc.) |
| `LOG_FORMAT` | `pretty` | `json` | Output format |
| `LOG_SERVICE_NAME` | `stg-backend` | `stg-backend` | Service label on every line |
| `RUST_ENV` | `development` | `production` | Environment label |
| `IMAGE_TAG` | — | deploy tag | Appended to version field |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | unset | unset | Reserved for future OpenTelemetry export |

## Log schema

Every JSON log line includes (when applicable):

| Field | Description |
|-------|-------------|
| `timestamp` | ISO 8601 time |
| `level` | `DEBUG`, `INFO`, `WARN`, `ERROR` (FATAL uses ERROR + `fatal: true`) |
| `service` | Application name (`stg-backend`) |
| `environment` | `development`, `production`, `test` |
| `version` | Crate version + optional `IMAGE_TAG` |
| `request_id` | Per-request UUID (also in `x-request-id` response header) |
| `trace_id` | From W3C `traceparent` when present |
| `user_id` | Authenticated user email (never session tokens) |
| `event` | Typed event name (`http.request`, `auth.login.success`, `db.connect`, …) |
| `error.code` / `error.message` | API and operational errors |

### HTTP access log (`event = "http.request"`)

```json
{
  "event": "http.request",
  "http.method": "POST",
  "http.path": "/api/contests",
  "http.status_code": 500,
  "duration_ms": 842,
  "request_id": "a1b2c3d4-...",
  "user_id": "player@example.com",
  "client_ip": "10.0.0.1"
}
```

## Redaction policy

Never logged: passwords, API keys, `Authorization` headers, session IDs/tokens, cookies, or credentials in connection strings. Redis/DB URLs are logged as host:port only.

## LogQL examples (Grafana → Explore → Loki)

**Label note:** Promtail sets `container="stg-backend"` (Docker name) and `service="backend"` (Compose service). Do **not** use `{service="stg-backend"}` — that label value does not exist.

**Parse error?** Loki requires a non-empty label matcher. Always start with e.g. `{container="stg-backend"}`. Never run `{}` or `| json` alone.

```logql
{container="stg-backend"} | json | request_id="<uuid>"
```

```logql
{container="stg-backend"} | json | event="http.request" | user_id="player@example.com" | http_status_code >= 400
```

```logql
{container="stg-backend"} | json | level="ERROR"
```

```logql
{container="stg-backend"} | json | error_code="DATABASE_ERROR"
```

## Production incident runbook

**Prerequisites:** main stack + `deploy/docker-compose.observability.yml` running; Grafana on `${GRAFANA_PORT:-3000}`.

### User-reported bug (email + approximate time)

1. Grafana → **Explore** → **Loki**; set the time range around the incident.
2. Find the failing request:
   ```logql
   {container="stg-backend"} | json | event="http.request" | user_id="<email>" | http_status_code >= 400
   ```
3. Copy `request_id` from the access log line.
4. Trace the full request:
   ```logql
   {container="stg-backend"} | json | request_id="<id>"
   ```
5. Read `error.code`, `error.message`, `db.*`, and `scheduler.*` events in that result set.
6. Confirm scope in **Prometheus**: 5xx rate, latency, scheduler metrics.

### Site-wide outage

1. `{container="stg-backend"} | json | level=~"ERROR|fatal"` (last 5–15 minutes)
2. `curl https://<host>/health/detailed` on the live server
3. Check `event=~"service\\..*|db\\..*|config\\..*"` around the last deploy/restart
4. Compare Prometheus 5xx rate to baseline

### Cheat sheet

| Anchor | LogQL |
|--------|-------|
| Request ID (`x-request-id` header) | `{container="stg-backend"} \| json \| request_id="..."` |
| User + route | `user_id="..."` + `http_path=~"/api/..."` |
| Error code | `error_code="DATABASE_ERROR"` |
| All 500s on a route | `http_path=~"/api/contests.*"` + `http_status_code=500` |

### Temporary verbose logging

Set targeted filter, restart backend, reproduce, then revert:

```bash
RUST_LOG=backend::contest=debug,backend::db=debug
```

## Disk and retention

- **Docker containers:** `json-file` driver with `max-size: 50m`, `max-file: 5` (see `deploy/docker-compose.yml`).
- **Loki:** 14-day retention (`deploy/observability/loki-config.yml`).
- **Prometheus:** 14-day TSDB retention.

## OpenTelemetry (future)

Set `OTEL_EXPORTER_OTLP_ENDPOINT` when an OTLP collector is available. The backend honors incoming W3C `traceparent` for `trace_id` correlation today; full distributed tracing export is a follow-up.

## Code layout

| Path | Role |
|------|------|
| `back/api/src/observability/` | Init, context, redaction, typed events |
| `back/api/src/middleware.rs` | HTTP access logs + correlation |
| `back/api/src/error.rs` | Central API error logging |
| `deploy/docker-compose.observability.yml` | Loki, Promtail, Grafana, Prometheus |
| `scripts/start-observability.sh` | Start observability stack |

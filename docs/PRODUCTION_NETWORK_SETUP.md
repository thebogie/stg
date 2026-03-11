# Production Network Setup Summary

## Your Current Setup ✅

### External Traffic Flow
```
Internet → Squarespace DNS → Traefik (Proxmox) → 192.168.1.51:50003 (Frontend)
```

### Traefik Routing (Must Be Configured)
Traefik on your Proxmox server must route:
- `https://smacktalkgaming.com/` → `192.168.1.51:50003` (Frontend)
- `https://smacktalkgaming.com/api/*` → `192.168.1.51:50002` (Backend)

### Container Setup on 192.168.1.51

**Port Mappings:**
- **ArangoDB**: Host `50001` → Container `8529`
- **Backend**: Host `50002` → Container `50002`
- **Frontend**: Host `50003` → Container `50003`
- **Redis**: Host `63791` → Container `6379`

**Docker Network Communication:**
- Backend → ArangoDB: `http://arangodb:8529` (uses Docker service name)
- Backend → Redis: `redis://redis:6379/` (uses Docker service name)

## How It Works

### 1. User Visits Website
- User goes to `https://smacktalkgaming.com`
- Traefik receives HTTPS request
- Traefik routes to `192.168.1.51:50003` (Frontend container)

### 2. Frontend Loads
- Frontend container serves static files (HTML, WASM, CSS, JS)
- Browser receives frontend app

### 3. Frontend Makes API Calls
- Frontend JavaScript uses **relative URLs**: `/api/players/login`
- Browser sends request to: `https://smacktalkgaming.com/api/players/login`
- Traefik routes `/api/*` to `192.168.1.51:50002` (Backend container)

### 4. Backend Processes Request
- Backend receives request on port `50002`
- Backend binds to `0.0.0.0:50002` (accessible from host)
- Backend connects to ArangoDB using `http://arangodb:8529` (Docker service name)
- Backend connects to Redis using `redis://redis:6379/` (Docker service name)

### 5. Response Flows Back
- Backend → Traefik → Browser

## Verification Checklist

### ✅ Already Configured
- [x] Frontend uses relative URLs (no hardcoded backend URL)
- [x] Backend uses Docker service names for database connections
- [x] Port mappings are correct in `docker-compose.stg_prod.yml`
- [x] `SERVER_HOST=0.0.0.0` is set in backend environment

### ⚠️ Must Verify on Traefik
- [ ] Traefik routes `/` → `192.168.1.51:50003`
- [ ] Traefik routes `/api/*` → `192.168.1.51:50002`
- [ ] SSL/TLS certificates are valid
- [ ] Traefik forwards correct headers (Host, X-Forwarded-Proto, etc.)

## Testing Your Setup

### Test from Production Server (Internal)
```bash
# Test frontend directly
curl -I http://localhost:50003/

# Test backend directly
curl http://localhost:50002/health

# Test ArangoDB directly
curl http://localhost:50001/_api/version
```

### Test from Browser (External via Traefik)
```bash
# Test frontend via Traefik
curl -I https://smacktalkgaming.com/

# Test backend via Traefik
curl https://smacktalkgaming.com/api/health
```

### Test from Container (Internal Docker Network)
```bash
# Backend → ArangoDB
docker exec backend wget -q -O- http://arangodb:8529/_api/version

# Backend → Redis
docker exec backend redis-cli -h redis ping

# Frontend → Backend (should fail - frontend doesn't have wget)
# Instead, test from browser console:
# fetch('/api/health').then(r => r.json()).then(console.log)
```

## Common Issues

### Issue: Frontend Can't Reach Backend
**Symptom:** Browser console shows CORS errors or connection refused
**Cause:** Traefik not routing `/api/*` to backend
**Fix:** Update Traefik config to route `/api/*` → `192.168.1.51:50002`

### Issue: Backend Can't Connect to ArangoDB
**Symptom:** Backend logs show "Connection refused" to ArangoDB
**Cause:** Backend trying to use host port instead of service name
**Fix:** Ensure `ARANGO_URL=http://arangodb:8529` is set (already configured)

### Issue: Backend Not Accessible from Host
**Symptom:** `curl http://localhost:50002/health` fails
**Cause:** Backend binding to `localhost` instead of `0.0.0.0`
**Fix:** Ensure `SERVER_HOST=0.0.0.0` is set (already configured in compose file)

## Next Steps

1. **Run test-and-push script** to rebuild containers with latest fixes:
   ```bash
   ./scripts/test-and-push-prod.sh
   ```

2. **On production server, pull and restart:**
   ```bash
   cd ~/stg/repo
   ./scripts/prod-compose.sh pull
   ./scripts/prod-compose.sh up -d
   ```

3. **Verify Traefik routing:**
   - Visit `https://smacktalkgaming.com` (should load frontend)
   - Open browser console, run: `fetch('/api/health').then(r => r.json()).then(console.log)`
   - Should return: `{status: "ok", timestamp: ..., version: "..."}`

4. **Check container health:**
   ```bash
   docker ps --format "table {{.Names}}\t{{.Status}}"
   ```
   All containers should show `healthy` status.


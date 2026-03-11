# Dev Container (Docker in WSL)

This dev container gives you a consistent Rust + Node environment and **uses Docker from your host**. When you open the project **inside WSL** and then "Reopen in Container", the container mounts WSL's Docker socket, so all Docker commands run **in WSL** (SurrealDB, Redis, backend).

## Setup

1. **Use WSL for the project**
   - Open the repo from a WSL path in Cursor/VS Code, e.g. `\\wsl$\Ubuntu\home\...\stg` or `~/work/stg` from the WSL terminal.
   - Ensure Docker is running in WSL (see **Verify Docker Engine in WSL** below).

2. **Reopen in Dev Container**
   - Command Palette → **Dev Containers: Reopen in Container**.
   - Wait for the container to build (first time may take a few minutes).

3. **Run the stack (all Docker in WSL)**
   - Terminal 1 (backend): `./scripts/start-back.sh` or `./scripts/start-back.sh --no-build`
   - Terminal 2 (frontend): `./scripts/start-front.sh`

SurrealDB, Redis, and the backend container will start **in WSL**; you edit and run commands inside the dev container.

## Why it works

- The **docker-outside-of-docker** feature installs the Docker CLI and mounts the **host's** Docker socket.
- When the "host" is WSL (because you opened the folder in WSL), that socket is WSL's Docker daemon.
- So `docker compose` and `./scripts/start-back.sh` run containers in WSL, not inside the dev container.

## Verify Docker Engine in WSL

**From a WSL terminal** (not inside the dev container yet):

1. **Is Docker running?**
   ```bash
   docker info
   ```
   - If it prints server/version info and no "Cannot connect" error, the daemon is running.

2. **Where is the daemon?**
   ```bash
   docker context show
   docker version
   ```
   - **Docker Engine in WSL:** You installed Docker inside WSL (e.g. `apt install docker.io` + `sudo service docker start`, or the official Docker Engine for Linux in WSL). Context is usually `default`; `docker version` shows `Server` with the Linux engine version.
   - **Docker Desktop (WSL 2 backend):** Context is often `desktop-linux`. The daemon still runs in a WSL 2 VM that Docker Desktop manages; from WSL's point of view it's "Docker in WSL" and the socket is in WSL, so the dev container will use it the same way.

3. **Quick run test**
   ```bash
   docker run --rm hello-world
   ```
   If this succeeds, Docker in WSL is working. After you "Reopen in Container", the same `docker` (via the mounted socket) will run containers in that same WSL engine.

**Install Docker Engine in WSL (no Docker Desktop):**  
If `sudo service docker start` says "Unit docker.service not found", install the engine first. On Ubuntu/WSL:

```bash
sudo apt-get update && sudo apt-get install -y ca-certificates curl gnupg
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt-get update
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
sudo service docker start
sudo usermod -aG docker $USER
```

Then log out of WSL and open a new terminal (or run `newgrp docker`). For other distros: [Install Docker Engine](https://docs.docker.com/engine/install/).

Optional: start Docker when you open a WSL terminal by adding to `~/.bashrc`:
```bash
if ! docker info &>/dev/null; then sudo service docker start 2>/dev/null; fi
```

## Optional

- **Config:** Copy `config/env.dev.template` to `config/.env.dev` and adjust if needed (see repo root `config/`).
- **Verify SurrealDB:** From inside the dev container, `./scripts/verify-surreal-local.sh` (SurrealDB must be running).

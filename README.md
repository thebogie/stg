# STG_RD - Gaming Platform for Tournaments and Competitions

A comprehensive gaming platform built with Rust for managing tournaments, competitions, and player analytics.

## Features

### Core Functionality
- **Tournament Management**: Create, manage, and track gaming tournaments
- **Player Analytics**: Comprehensive player statistics and performance tracking
- **Venue Management**: Gaming venue discovery and management
- **Game Database**: Integration with BoardGameGeek API for game information
- **Rating System**: Glicko2-based rating system for competitive play

### Authentication & User Management
- **Player Registration & Login**: Secure user account creation and authentication
- **Session Management**: Redis-based session storage with secure cookies
- **Profile Updates**: Players can update their email, handle, and password
- **Security Features**: 
  - Argon2 password hashing
  - CSRF protection
  - Rate limiting on login attempts
  - Secure session cookies

## API Endpoints

### Authentication
- `POST /api/players/register` - Player registration
- `POST /api/players/login` - Player authentication
- `POST /api/players/logout` - Session termination
- `GET /api/players/me` - Get current player profile
- `PUT /api/players/me/email` - Update email address
- `PUT /api/players/me/handle` - Update username/handle
- `PUT /api/players/me/password` - Update password

### Other Endpoints
- `GET /api/players/search` - Search for players
- `GET /api/venues` - Venue information
- `GET /api/games` - Game database
- `GET /api/contests` - Contest information
- `GET /api/analytics` - Player and contest analytics

## Technology Stack

- **Backend**: Rust with Actix-web (`back/api`, package name `backend`)
- **Database**: SurrealDB (document database)
- **Cache**: Redis for session storage
- **Frontend**: WebAssembly with Yew (`front/web`); Tauri desktop/mobile (`front/tauri`)
- **Authentication**: Session-based with secure cookies
- **Password Security**: Argon2 (or bcrypt) hashing

## Getting Started

### Prerequisites
- Rust 1.70+
- Docker and Docker Compose
- Git

### Quick Start

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd stg_rd
   ```

2. **Setup environment**
   ```bash
   ./config/setup-env.sh dev    # creates config/.env.dev
   ./config/setup-env.sh prod   # creates config/.env.prod
   # Edit the created file(s) as needed
   ```

3. **Run backend (terminal 1)** – SurrealDB + Redis + API
   ```bash
   ./scripts/start-back.sh
   ```

4. **Run frontend (terminal 2)** – Yew in browser or Tauri window
   ```bash
   ./scripts/start-front.sh    # or: ./scripts/start-tauri.sh
   ```

5. **Access** (default ports: backend 50002, frontend 50003; SurrealDB 50001)
   ```bash
   source scripts/load-env.sh
   echo "Frontend: http://localhost:${FRONTEND_PORT}"
   echo "Backend:  http://localhost:${BACKEND_PORT}"
   ```

For full setup and workflows, see **[docs/README.md](docs/README.md)** and **[docs/WORKFLOW.txt](docs/WORKFLOW.txt)**. Production deploy: **[docs/GHCR_SETUP.md](docs/GHCR_SETUP.md)**.

## Documentation

- **[Documentation index](docs/README.md)** – Start here for all docs
- **[Project structure](docs/PROJECT_STRUCTURE.md)** – Repo layout and run commands
- **[Workflow](docs/WORKFLOW.txt)** – CI, backend, frontend, production (one page)
- **[Development setup](docs/setup/DEVELOPMENT_SETUP.md)** – Environment and local run
- **[Testing](docs/testing/HOW_TO_RUN_TESTS.md)** – How to run tests
- **[Deployment (GHCR)](docs/GHCR_SETUP.md)** – Build on push, pull on production

## Development

### Project structure
```
├── back/api/         # Rust backend API (package: backend)
├── front/web/        # Yew (WASM) web app
├── front/tauri/      # Tauri desktop/mobile
├── shared/           # Shared types and DTOs
├── testing/          # Integration tests
├── config/           # Env files (.env.dev, .env.prod)
├── deploy/           # Docker Compose (SurrealDB, Redis, backend)
├── scripts/          # start-back, start-front, ci, etc.
└── docs/             # Documentation
```

### Running tests
```bash
./ci-local.sh all                  # build, unit, integration, e2e
./ci-local.sh unit                 # unit only
cargo nextest run -p backend      # backend unit tests
```

### Code Quality
- Uses `cargo fmt` for code formatting
- Uses `cargo clippy` for linting
- Comprehensive test coverage
- Type-safe API with validation

## Security

- All passwords are hashed using Argon2
- Session cookies are HTTP-only and secure
- CSRF protection enabled
- Rate limiting on authentication endpoints
- Input validation on all endpoints
- Secure cookie settings for production

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## License

MIT License - see LICENSE file for details

## Support

For questions or support, please open an issue on GitHub or contact the development team.

---

**Note**: This platform is designed for gaming tournaments and competitions. All authentication features are production-ready with industry-standard security practices.


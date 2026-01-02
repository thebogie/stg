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

- **Backend**: Rust with Actix-web framework
- **Database**: ArangoDB (document database)
- **Cache**: Redis for session storage
- **Frontend**: WebAssembly with Yew framework
- **Authentication**: Session-based with secure cookies
- **Password Security**: Argon2 hashing algorithm

## Getting Started

### Prerequisites
- Rust 1.70+
- Docker and Docker Compose
- Git

### Quick Start

**For detailed setup instructions, see [README_QUICK_START.md](README_QUICK_START.md)**

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd stg_rd
   ```

2. **Setup environment**
   ```bash
   # For development
   ./config/setup-env.sh development
   # Edit config/.env.development with your values
   
   # For production
   ./config/setup-env.sh production
   # Edit config/.env.production with your values
   ```

3. **Start services**
   ```bash
   # Development
   ./deploy/deploy.sh --env development --build
   
   # Production
   ./deploy/deploy.sh --env production --build
   ```

4. **Access the application** (ports from your `.env` file)
   ```bash
   # Check your ports
   source scripts/load-env.sh  # or 'production'
   echo "Frontend: http://localhost:${FRONTEND_PORT}"
   echo "Backend: http://localhost:${BACKEND_PORT}"
   echo "ArangoDB: http://localhost:${ARANGODB_PORT}"
   echo "Redis: localhost:${REDIS_PORT}"
   ```

For production deployment, see [Production Deployment Guide](deploy/PRODUCTION_DEPLOYMENT.md).

## Documentation

- **[Quick Start Guide](README_QUICK_START.md)** - Get up and running quickly
- **[Hybrid Development](HYBRID_DEV_QUICK_START.md)** - Fast development with debugger support
- **[CI/CD Workflow](docs/CI_CD_WORKFLOW.md)** - Complete workflow from development to production
- **[Test-Then-Deploy Workflow](README_WORKFLOW.md)** - Production deployment workflow
- **[Development Setup](docs/setup/DEVELOPMENT_SETUP.md)** - Detailed development setup
- **[Project Structure](docs/setup/PROJECT_STRUCTURE.md)** - Project organization
- **[Testing Guide](docs/testing/TESTING_SETUP.md)** - Testing documentation
- **[Production Deployment](deploy/PRODUCTION_DEPLOYMENT.md)** - Production deployment guide
- **[Full Documentation Index](docs/README.md)** - Complete documentation index

## Development

### Project Structure
```
├── backend/          # Rust backend API
├── frontend/         # WebAssembly frontend
├── shared/           # Shared models and DTOs
├── migrations/       # Database migrations
├── testing/          # Integration tests
├── docs/             # Documentation (organized by category)
├── config/           # Environment configuration
└── deploy/            # Deployment configuration
```

### Running Tests
```bash
# Run all tests
cargo test --workspace

# Run specific package tests
cargo test --package backend
cargo test --package shared
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


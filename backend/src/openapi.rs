use utoipa::OpenApi;
use crate::error::ApiError;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::health::health_check,
        crate::health::detailed_health_check,
    ),
    components(schemas(
        crate::health::HealthResponse,
        ApiError,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "players", description = "Player management and authentication"),
        (name = "venues", description = "Venue management"),
        (name = "games", description = "Game management"),
        (name = "contests", description = "Contest management"),
        (name = "analytics", description = "Analytics and statistics"),
    ),
    info(
        title = "STG_RD Gaming Platform API",
        description = "A comprehensive gaming platform API for managing tournaments, competitions, and player analytics.\n\n## Authentication\n\nMost endpoints require authentication via Bearer token in the Authorization header:\n\n```\nAuthorization: Bearer <session_id>\n```\n\nGet a session_id by logging in via `/api/players/login`.",
        version = "0.2.11",
        contact(
            name = "API Support",
            email = "thebogie@example.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "http://localhost:3000", description = "Alternative development port"),
    )
)]
pub struct ApiDoc;

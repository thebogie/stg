# Venue Module

This module follows the same pattern as the player module and provides CRUD operations for venues.

## Structure

- `controller.rs` - HTTP handlers for venue endpoints
- `repository.rs` - Data access layer for venue operations
- `usecase.rs` - Business logic layer for venue operations

## API Endpoints

### GET /api/venues
Get all venues

### GET /api/venues/{id}
Get a specific venue by ID

### POST /api/venues
Create a new venue

**Request Body:**
```json
{
  "displayName": "Venue Name",
  "formattedAddress": "123 Main St, City, State 12345",
  "placeId": "google_place_id",
  "lat": 40.7128,
  "lng": -74.0060
}
```

### PUT /api/venues/{id}
Update an existing venue

### DELETE /api/venues/{id}
Delete a venue

## Data Model

The venue module uses the `Venue` model from the shared crate:

```rust
pub struct Venue {
    pub id: String,
    pub rev: String,
    pub display_name: String,
    pub formatted_address: String,
    pub place_id: String,
    pub lat: f64,
    pub lng: f64,
}
```

## Repository Pattern

The module follows the repository pattern with:
- `VenueRepository` trait defining the interface
- `VenueRepositoryImpl` providing the concrete implementation
- Uses ArangoDB for data persistence

## Use Case Pattern

The module follows the use case pattern with:
- `VenueUseCase` trait defining business operations
- `VenueUseCaseImpl` providing the concrete implementation
- Handles validation and business logic

## Validation

The module uses the `validator` crate for input validation:
- Display name is required and must have minimum length
- Formatted address is required and must have minimum length
- Place ID is required and must have minimum length
- Latitude and longitude are required as f64 values 
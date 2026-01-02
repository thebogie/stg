# Authentication API Documentation

This document describes the authentication endpoints available in the STG_RD gaming platform.

## Overview

The authentication system uses session-based authentication with Redis for session storage. Players can register, login, logout, and update their profile information.

## Base URL

All endpoints are prefixed with `/api/players`

## Public Endpoints

### Player Registration

**POST** `/api/players/register`

Register a new player account.

**Request Body:**
```json
{
  "username": "player_handle",
  "email": "player@example.com",
  "password": "secure_password123"
}
```

**Response:**
```json
{
  "id": "player/1234567890",
  "firstname": "player_handle",
  "handle": "player_handle",
  "email": "player@example.com",
  "createdAt": "2024-01-01T00:00:00Z",
  "isAdmin": false
}
```

### Player Login

**POST** `/api/players/login`

Authenticate a player and create a session.

**Request Body:**
```json
{
  "email": "player@example.com",
  "password": "secure_password123"
}
```

**Response:**
```json
{
  "player": {
    "id": "player/1234567890",
    "firstname": "player_handle",
    "handle": "player_handle",
    "email": "player@example.com",
    "createdAt": "2024-01-01T00:00:00Z",
    "isAdmin": false
  },
  "session_id": "uuid-session-id"
}
```

**Authentication:**
The frontend stores the `session_id` in LocalStorage and sends it in the Authorization header as `Bearer <session_id>` for all subsequent requests.

### Player Logout

**POST** `/api/players/logout`

End the current session and clear cookies.

**Request:** No body required, uses Authorization header with Bearer token for authentication.

**Response:** `200 OK` with message "Logged out"

### Search Players

**GET** `/api/players/search?query=search_term`

Search for players by handle or email.

**Query Parameters:**
- `query`: Search term (required)

**Response:**
```json
[
  {
    "id": "player/1234567890",
    "firstname": "player_handle",
    "handle": "player_handle",
    "email": "player@example.com",
    "createdAt": "2024-01-01T00:00:00Z",
    "isAdmin": false
  }
]
```

## Protected Endpoints

All protected endpoints require authentication via the Authorization header with Bearer token.

### Get Current Player Profile

**GET** `/api/players/me`

Get the current authenticated player's profile.

**Response:**
```json
{
  "id": "player/1234567890",
  "firstname": "player_handle",
  "handle": "player_handle",
  "email": "player@example.com",
  "createdAt": "2024-01-01T00:00:00Z",
  "isAdmin": false
}
```

### Update Email Address

**PUT** `/api/players/me/email`

Update the current player's email address.

**Request Body:**
```json
{
  "email": "newemail@example.com",
  "password": "current_password"
}
```

**Response:**
```json
{
  "message": "Email updated successfully",
  "player": {
    "id": "player/1234567890",
    "firstname": "player_handle",
    "handle": "player_handle",
    "email": "newemail@example.com",
    "createdAt": "2024-01-01T00:00:00Z",
    "isAdmin": false
  }
}
```

### Update Handle/Username

**PUT** `/api/players/me/handle`

Update the current player's handle/username.

**Request Body:**
```json
{
  "handle": "new_handle",
  "password": "current_password"
}
```

**Response:**
```json
{
  "message": "Handle updated successfully",
  "player": {
    "id": "player/1234567890",
    "firstname": "new_handle",
    "handle": "new_handle",
    "email": "player@example.com",
    "createdAt": "2024-01-01T00:00:00Z",
    "isAdmin": false
  }
}
```

### Update Password

**PUT** `/api/players/me/password`

Update the current player's password.

**Request Body:**
```json
{
  "current_password": "old_password",
  "new_password": "new_secure_password123"
}
```

**Response:**
```json
{
  "message": "Password updated successfully",
  "player": {
    "id": "player/1234567890",
    "firstname": "player_handle",
    "handle": "player_handle",
    "email": "player@example.com",
    "createdAt": "2024-01-01T00:00:00Z",
    "isAdmin": false
  }
}
```

## Error Responses

All endpoints return appropriate HTTP status codes and error messages:

### 400 Bad Request
```json
{
  "error": "Bad Request",
  "message": "Validation error details"
}
```

### 401 Unauthorized
```json
{
  "error": "Unauthorized",
  "message": "Authentication required or invalid credentials"
}
```

### 404 Not Found
```json
{
  "error": "Not Found",
  "message": "Player not found"
}
```

### 409 Conflict
```json
{
  "error": "Conflict",
  "message": "Email or handle already exists"
}
```

### 429 Too Many Requests
```json
{
  "error": "Too Many Requests",
  "message": "Too many login attempts. Please try again later."
}
```

## Security Features

1. **Password Hashing**: All passwords are hashed using Argon2
2. **Session Management**: Secure session cookies with HTTP-only and SameSite flags
3. **CSRF Protection**: CSRF tokens for form submissions
4. **Rate Limiting**: Login attempts are rate-limited per IP address
5. **Input Validation**: All inputs are validated using the validator crate
6. **Secure Cookies**: Cookies are marked as secure in production environments

## Usage Examples

### JavaScript/Frontend

```javascript
// Login
const response = await fetch('/api/players/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'player@example.com',
    password: 'password123'
  })
});

// Update email (requires authentication)
const updateResponse = await fetch('/api/players/me/email', {
  method: 'PUT',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'newemail@example.com',
    password: 'current_password'
  })
});
```

### cURL Examples

```bash
# Register
curl -X POST http://localhost:8080/api/players/register \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","email":"test@example.com","password":"password123"}'

# Login
curl -X POST http://localhost:8080/api/players/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"password123"}' \
  -c cookies.txt

# Update email (using session cookie)
curl -X PUT http://localhost:8080/api/players/me/email \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{"email":"newemail@example.com","password":"password123"}'
```

## Notes

- All timestamps are in ISO 8601 format with timezone information
- Handle/username must be 3-50 characters and contain only letters, numbers, and underscores
- Passwords must be at least 8 characters long
- Email addresses are validated for proper format
- Session cookies are automatically managed by the browser
- The `isAdmin` field is managed by administrators and cannot be changed by regular players

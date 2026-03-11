# Admin Authorization System

## Overview

The stg_rd project now includes a comprehensive admin authorization system that protects administrative features like the Glicko2 ratings scheduler. This system ensures that only authorized players can access sensitive administrative functions.

## Architecture

### 1. Player Model Updates

#### New `isAdmin` Field
All players now have an `isAdmin` boolean field that determines their administrative privileges:

```rust
pub struct Player {
    // ... existing fields ...
    
    /// Whether the player has administrative privileges
    #[serde(rename = "isAdmin")]
    pub is_admin: bool,
}
```

#### Database Schema
The `player` collection now includes:
- `isAdmin`: Boolean field (defaults to `false`)
- All existing players are automatically set to non-admin during migration

### 2. Admin Authorization Middleware

#### `AdminAuthMiddleware`
A new middleware layer that extends the existing authentication system:

```rust
pub struct AdminAuthMiddleware {
    pub redis: Rc<redis::Client>,
    pub db: Rc<Database<ReqwestClient>>,
}
```

**Features:**
- **Dual Authentication**: First authenticates the user, then checks admin status
- **Database Verification**: Queries the database to verify admin privileges
- **Comprehensive Logging**: Logs all admin access attempts
- **Error Handling**: Provides clear error messages for unauthorized access

#### How It Works
1. **Session Validation**: Checks if the user has a valid session
2. **Player Lookup**: Retrieves player information from the database
3. **Admin Check**: Verifies the `isAdmin` field is `true`
4. **Access Control**: Allows or denies access based on admin status

### 3. Protected Endpoints

#### Scheduler Management
The following endpoints are protected by admin authorization:

```http
GET  /api/ratings/scheduler/status    # Get scheduler status
POST /api/ratings/scheduler/trigger   # Trigger manual recalculation
```

#### Health Checks
Admin-specific health endpoints:

```http
GET /health/scheduler                  # Scheduler health status
```

## Setup and Configuration

### 1. Database Migration

#### Run the Migration
```bash
# Navigate to migrations directory
cd migrations

# Run the migration to add isAdmin field
./run-migrations.sh
```

#### Migration Details
The migration `20250115T000000_add_admin_field_to_players.aql`:
- Adds `isAdmin: false` to all existing players
- Ensures no players have admin access by default
- Logs the number of players updated

### 2. Setting an Admin Player

#### Using the Admin Script
```bash
# Make sure you're in the stg_rd project directory
cd /path/to/stg_rd

# Set a player as admin (replace with actual email)
./scripts/set_admin_player.sh admin@example.com
```

#### Manual Database Update
```aql
// Set a specific player as admin
FOR p IN player
  FILTER p.email == "admin@example.com"
  UPDATE p WITH { isAdmin: true } IN player
  RETURN p
```

#### Environment Variables
```bash
# Required for the admin script
ARANGO_URL=http://localhost:8529
ARANGO_DB=stg_rd
ARANGO_USERNAME=root
ARANGO_PASSWORD_FILE=.password
```

### 3. Frontend Integration

#### Admin Status Check
The frontend automatically checks admin status:

```rust
impl AuthState {
    /// Check if the current player has administrative privileges
    pub fn is_admin(&self) -> bool {
        self.player.as_ref().map(|p| p.is_admin).unwrap_or(false)
    }
}
```

#### Conditional Rendering
Admin components are only shown to authorized users:

```rust
// Scheduler Monitor Section (Admin Only)
if auth.is_admin() {
    <div class="dashboard-section">
        <SchedulerMonitor />
    </div>
}
```

## Usage

### 1. Accessing Admin Features

#### For Admin Users
1. **Log in** with an admin account
2. **Navigate** to the Analytics Dashboard
3. **View** the Scheduler Monitor section
4. **Monitor** and **control** the Glicko2 ratings scheduler

#### For Non-Admin Users
- Admin features are **hidden** from the UI
- API calls to admin endpoints return **401 Unauthorized**
- Clear error messages explain the access restriction

### 2. Admin Functions

#### Scheduler Monitoring
- **Real-time Status**: View current scheduler state
- **Last Run Time**: See when recalculation last occurred
- **Next Scheduled Run**: Know when the next automatic run will happen

#### Manual Control
- **Trigger Recalculation**: Manually start ratings recalculation
- **Period Selection**: Specify exact time periods for recalculation
- **Immediate Execution**: Bypass the monthly schedule when needed

### 3. Security Features

#### Authentication Required
- All admin endpoints require valid authentication
- Session tokens must be current and valid
- Redis session storage ensures secure session management

#### Admin Verification
- Database queries verify admin status
- No client-side admin flags can bypass security
- Comprehensive logging of all admin actions

## Monitoring and Logging

### 1. Access Logs

#### Successful Admin Access
```
INFO  AdminAuthMiddleware: Player player_123 is admin, allowing access
```

#### Failed Admin Access
```
WARN  AdminAuthMiddleware: Player player_456 is not admin, denying access
```

#### Authentication Failures
```
WARN  AdminAuthMiddleware: No session ID found for POST /api/ratings/scheduler/trigger
```

### 2. Health Monitoring

#### Scheduler Health
```bash
# Check scheduler health
curl "http://localhost:8080/health/scheduler"

# Response
{
  "status": "ok",
  "timestamp": 1704067200,
  "message": "Glicko2 ratings scheduler is running in the backend",
  "note": "Check /api/ratings/scheduler/status for detailed scheduler information"
}
```

#### Admin Endpoint Health
```bash
# Check admin endpoint (requires admin authentication)
curl -H "Authorization: Bearer YOUR_SESSION_TOKEN" \
     "http://localhost:8080/api/ratings/scheduler/status"
```

## Troubleshooting

### Common Issues

#### 1. **Admin Features Not Visible**
**Symptoms**: Scheduler monitor doesn't appear in analytics dashboard
**Solutions**:
- Verify the player has `isAdmin: true` in the database
- Check that the player is logged in
- Ensure the frontend has refreshed after login

#### 2. **401 Unauthorized Errors**
**Symptoms**: API calls to admin endpoints return 401
**Solutions**:
- Verify the session token is valid
- Check if the player has admin privileges
- Ensure the session hasn't expired

#### 3. **Migration Errors**
**Symptoms**: Database migration fails
**Solutions**:
- Check ArangoDB connection and credentials
- Verify the player collection exists
- Check migration script permissions

### Debug Commands

#### Check Player Admin Status
```bash
# Query player admin status
curl -u "root:password" \
     "http://localhost:8529/_db/stg_rd/_api/aql" \
     -H "Content-Type: application/json" \
     -d '{
       "query": "FOR p IN player FILTER p.email == \"admin@example.com\" RETURN { handle: p.handle, isAdmin: p.isAdmin }"
     }'
```

#### Verify Admin Middleware
```bash
# Check if admin middleware is working
curl -H "Authorization: Bearer INVALID_TOKEN" \
     "http://localhost:8080/api/ratings/scheduler/status"
# Should return 401 Unauthorized
```

#### Test Admin Access
```bash
# Test with valid admin session
curl -H "Authorization: Bearer VALID_ADMIN_TOKEN" \
     "http://localhost:8080/api/ratings/scheduler/status"
# Should return scheduler status
```

## Security Considerations

### 1. **Principle of Least Privilege**
- Admin access is **explicitly granted** to specific players
- **No default admin accounts** are created
- **Regular users** cannot access admin features

### 2. **Session Security**
- **Redis-based sessions** provide secure token storage
- **Session expiration** ensures temporary access
- **Token validation** prevents session hijacking

### 3. **Database Security**
- **Admin checks** are performed at the database level
- **No client-side bypass** of admin restrictions
- **Comprehensive logging** of all admin actions

### 4. **API Security**
- **Authentication required** for all admin endpoints
- **Authorization verified** before processing requests
- **Clear error messages** without information leakage

## Future Enhancements

### 1. **Role-Based Access Control**
- **Multiple admin levels** (super admin, moderator, etc.)
- **Granular permissions** for different admin functions
- **Permission inheritance** and delegation

### 2. **Admin Audit Trail**
- **Detailed logging** of all admin actions
- **Change tracking** for admin privilege modifications
- **Admin activity reports** and analytics

### 3. **Advanced Security**
- **Two-factor authentication** for admin accounts
- **IP whitelisting** for admin access
- **Session timeout** configuration for admin sessions

---

**Last Updated**: January 2025
**Version**: 1.0.0
**Status**: Production Ready ✅
**Security Level**: Admin-Protected 🔒

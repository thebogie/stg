# Venue Place ID Migration Tool

This tool helps populate missing `place_id` values in your ArangoDB venue collection using the Google Places API.

## Overview

The `place_id` field in your venue documents is a unique identifier from Google Maps API that helps with:
- Accurate venue identification
- Consistent venue data across your application
- Integration with Google Maps services
- Avoiding duplicate venues

## Prerequisites

1. **Google Places API Key**: You need a valid Google Places API key
   - Get one from [Google Cloud Console](https://console.cloud.google.com/)
   - Enable the Places API service
   - Set up billing (required for API usage)

2. **ArangoDB Access**: Ensure you have access to your ArangoDB instance

## Installation

The migration tool is already included in your project. No additional installation is needed.

## Usage

### Quick Start

1. **Add your Google Places API key to your environment file**:
   ```bash
   # Add to .env.development or .env.production
   GOOGLE_LOCATION_API=your_google_places_api_key_here
   ```

2. **Run the migration**:
   ```bash
   ./scripts/run-place-id-migration.sh --env development
   ```

### Dry Run (Recommended First Step)

Before running the actual migration, test it with a dry run:

```bash
./scripts/run-place-id-migration.sh --env development --dry-run
```

This will show you what venues would be updated without making any changes.

### Environment Files

The script automatically loads configuration from environment files:

- **Development**: `../.env.development`
- **Production**: `../.env.production`

Required variables in your environment file:
```bash
GOOGLE_LOCATION_API=your_google_places_api_key_here
ARANGO_URL=http://localhost:50001
ARANGO_DB=your_database_name
ARANGO_USERNAME=your_username
ARANGO_PASSWORD=your_password
```

### Manual Execution

If you prefer to run the migration manually:

```bash
cd scripts
cargo build --release
cargo run --release --bin migrate-place-ids -- --env development --dry-run
```

## How It Works

1. **Query Venues**: Finds all venues with null, empty, or placeholder `place_id` values (preserves existing valid place_ids)
2. **Google Places Search**: For each venue, searches Google Places API using venue name + address
3. **Update Database**: Updates the venue document with the found `place_id`
4. **Rate Limiting**: Includes delays between API calls to respect Google's rate limits
5. **Error Handling**: Logs errors and continues processing other venues

### What Gets Updated

The migration will **only** update venues with these `place_id` values:
- `null` or empty string
- `"Unknown Place"`
- `"unknown"` or `"Unknown"`
- `"placeholder"` or `"temp"`

**Existing valid place_ids will be preserved** - the migration won't overwrite them.

## API Usage

The tool uses Google Places API **Text Search** endpoint:
- **Cost**: ~$0.017 per request
- **Rate Limit**: 100 requests per second
- **Quota**: 100,000 requests per day (free tier)

For 1000 venues, you can expect:
- **Cost**: ~$17 USD
- **Duration**: ~20 minutes (with rate limiting)

## Monitoring

The tool provides detailed logging:

```
[INFO] 🚀 Starting venue place_id migration
[INFO] 📊 Found 150 venues with missing place_id
[INFO] 🔄 Processing venue 1/150: 'Downtown Game Center'
[INFO] 🔍 Searching for place_id: 'Downtown Game Center 123 Main St, Downtown, DC 12345'
[INFO] ✅ Found place_id 'ChIJ...' for venue 'Downtown Game Center'
[INFO] ✅ Updated venue 'Downtown Game Center' with place_id: ChIJ...
[SUCCESS] Migration completed successfully
```

## Troubleshooting

### Common Issues

1. **API Key Invalid**:
   ```
   [ERROR] Google Places API returned status 'REQUEST_DENIED'
   ```
   - Check your API key is correct
   - Ensure Places API is enabled in Google Cloud Console
   - Verify billing is set up

2. **Rate Limiting**:
   ```
   [WARNING] Google Places API returned status 'OVER_QUERY_LIMIT'
   ```
   - Wait and retry later
   - Check your API quota usage

3. **No Results Found**:
   ```
   [WARNING] No place_id found for venue 'Some Venue'
   ```
   - Venue may not exist in Google Places
   - Try manually searching on Google Maps
   - Consider updating venue name/address

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug ./scripts/run-place-id-migration.sh
```

## Best Practices

1. **Always run dry-run first** to see what will be updated
2. **Monitor API costs** in Google Cloud Console
3. **Backup your database** before running the migration
4. **Run during off-peak hours** to minimize impact
5. **Review skipped venues** and handle them manually if needed

## Manual Venue Updates

For venues that couldn't be automatically updated, you can:

1. **Search manually on Google Maps** to find the correct place
2. **Update the venue** using your application's venue management interface
3. **Use the Google Places API directly** to get the place_id

Example manual update:
```sql
UPDATE @venue_id WITH { place_id: "ChIJ..." } IN venue
```

## Support

If you encounter issues:

1. Check the logs for specific error messages
2. Verify your API key and permissions
3. Test with a single venue first
4. Contact your development team for assistance

## Cost Estimation

| Venues | Estimated Cost | Duration |
|--------|----------------|----------|
| 100    | ~$1.70         | ~2 min   |
| 500    | ~$8.50         | ~10 min  |
| 1000   | ~$17.00        | ~20 min  |
| 5000   | ~$85.00        | ~1.5 hr  |

*Costs are approximate and may vary based on API usage patterns.*

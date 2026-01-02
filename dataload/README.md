# Data Loader

This program loads game session records from `stg_records.json` into an ArangoDB database.

## Prerequisites

- Rust and Cargo installed
- ArangoDB running locally or accessible via network
- The `stg_records.json` file in the program directory

## Environment Variables

Create a `.env` file in the top-level directory (one level above this folder) with the following variables:

```env
ARANGO_URL=http://localhost:8529
ARANGO_DB=stg_rd
ARANGO_USERNAME=root
ARANGO_PASSWORD=your_password
ARANGO_COLLECTION_PLAYERS=players
ARANGO_COLLECTION_GAMES=games
ARANGO_COLLECTION_VENUES=venues
ARANGO_COLLECTION_CONTESTS=contests
ARANGO_COLLECTION_OUTCOMES=outcomes
ARANGO_GRAPH=stg_graph
```

## Database Setup

Before running the program, ensure that:

1. The ArangoDB database exists
2. The collections exist:
   - players
   - games
   - venues
   - contests
   - outcomes
3. The graph exists with the name specified in ARANGO_GRAPH

You can create these using the ArangoDB web interface or arangosh.

## Building and Running

```bash
# Build the program
cargo build --release

# Run the program
cargo run --release
```

## Data Model

The program maps the JSON data to the following model:

- Venues: Locations where games are played
- Games: Board games played in sessions
- Players: People who participate in game sessions
- Contests: Game sessions with start/end times
- Outcomes: Results of players in contests

Relations are stored in the graph database:
- Player -> Contest: "resulted_in" with place and result

## Error Handling

The program logs errors and progress using the standard logging system. Check the output for any issues during data loading. 
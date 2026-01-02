#!/bin/bash

echo "Testing Player Analytics Endpoints..."
echo "====================================="

# Test platform analytics (should work)
echo "1. Testing platform analytics..."
curl -s "http://localhost:8080/api/analytics/platform" | jq '.total_players, .total_contests, .total_games, .total_venues' 2>/dev/null || echo "Platform analytics failed"

echo ""
echo "2. Testing player analytics endpoints..."

# Test players who beat me
echo "   - Players who beat me:"
curl -s "http://localhost:8080/api/analytics/player/opponents-who-beat-me" | jq '. | length' 2>/dev/null || echo "Failed"

# Test players I beat
echo "   - Players I beat:"
curl -s "http://localhost:8080/api/analytics/player/opponents-i-beat" | jq '. | length' 2>/dev/null || echo "Failed"

# Test game performance
echo "   - Game performance:"
curl -s "http://localhost:8080/api/analytics/player/game-performance" | jq '. | length' 2>/dev/null || echo "Failed"

# Test performance trends
echo "   - Performance trends:"
curl -s "http://localhost:8080/api/analytics/player/performance-trends" | jq '. | length' 2>/dev/null || echo "Failed"

echo ""
echo "Test completed!" 
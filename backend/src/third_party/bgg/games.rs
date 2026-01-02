use serde::{Deserialize, Serialize};
use shared::models::game::Game;
use anyhow::Result;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct BGGSearchResponse {
    items: Vec<BGGSearchItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGSearchItem {
    #[serde(rename = "type")]
    item_type: String,
    id: BGGId,
    name: BGGName,
    yearpublished: Option<BGGYearPublished>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGId {
    #[serde(rename = "value")]
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGName {
    #[serde(rename = "value")]
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGYearPublished {
    #[serde(rename = "value")]
    year: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGThingResponse {
    items: Vec<BGGThingItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGThingItem {
    #[serde(rename = "type")]
    item_type: String,
    id: String,
    name: Vec<BGGThingName>,
    description: Option<String>,
    yearpublished: Option<BGGYearPublished>,
    minplayers: Option<BGGMinPlayers>,
    maxplayers: Option<BGGMaxPlayers>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGThingName {
    #[serde(rename = "type")]
    name_type: String,
    #[serde(rename = "value")]
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGMinPlayers {
    #[serde(rename = "value")]
    min: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BGGMaxPlayers {
    #[serde(rename = "value")]
    max: String,
}

#[derive(Clone)]
pub struct BGGService {
    api_url: String,
    api_token: Option<String>,
    client: reqwest::Client,
}

impl BGGService {
    pub fn new() -> Result<Self> {
        let api_url = env::var("BGG_API_URL")
            .unwrap_or_else(|_| "https://boardgamegeek.com/xmlapi2".to_string());
        let api_token = env::var("BGG_API_TOKEN").ok();
        
        Ok(Self {
            api_url,
            api_token,
            client: reqwest::Client::new(),
        })
    }

    pub fn new_with_config(config: &crate::config::BGGConfig) -> Self {
        Self {
            api_url: config.api_url.clone(),
            api_token: config.api_token.clone(),
            client: reqwest::Client::new(),
        }
    }

    pub fn new_with_url(api_url: String) -> Self {
        Self {
            api_url,
            api_token: None,
            client: reqwest::Client::new(),
        }
    }

    /// Build a request with Authorization header if token is available
    fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.request(method, url);
        
        if let Some(token) = &self.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        request
    }

    pub async fn search_games(&self, query: &str) -> Result<Vec<Game>> {
        log::info!("Searching BGG API for query: '{}'", query);
        
        // Don't search if query is empty or too short
        if query.trim().is_empty() || query.trim().len() < 2 {
            log::info!("Query too short or empty, returning empty results");
            return Ok(Vec::new());
        }
        
        let search_url = format!("{}/search", self.api_url);
        let params = [
            ("query", query.trim()),
            ("type", "boardgame"),
            ("exact", "0"), // Allow partial matches
        ];

        log::info!("BGG API URL: {}", search_url);
        log::info!("BGG API params: {:?}", params);

        let response = self.build_request(reqwest::Method::GET, &search_url)
            .query(&params)
            .send()
            .await?;

        log::info!("BGG API response status: {}", response.status());

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("BGG Search API request failed: {}", response.status()));
        }

        let response_text = response.text().await?;
        log::info!("BGG search response length: {} characters", response_text.len());
        log::info!("BGG search response preview: {}", &response_text[..response_text.len().min(500)]);

        // Parse XML response
        let doc = roxmltree::Document::parse(&response_text)
            .map_err(|e| anyhow::anyhow!("Failed to parse BGG XML response: {}", e))?;
        
        let root = doc.root_element();
        if root.tag_name().name() != "items" {
            return Err(anyhow::anyhow!("Unexpected root element: {}", root.tag_name().name()));
        }

        let mut games = Vec::new();
        
        for item in root.children().filter(|n| n.is_element()) {
            if item.tag_name().name() != "item" {
                continue;
            }
            
            let id = item.attribute("id")
                .ok_or_else(|| anyhow::anyhow!("Missing id attribute"))?;
            
            let name_elem = item.children()
                .find(|n| n.is_element() && n.tag_name().name() == "name")
                .ok_or_else(|| anyhow::anyhow!("Missing name element"))?;
            
            let name = name_elem.attribute("value")
                .ok_or_else(|| anyhow::anyhow!("Missing name value"))?;
            
            let year_published = item.children()
                .find(|n| n.is_element() && n.tag_name().name() == "yearpublished")
                .and_then(|n| n.attribute("value"))
                .and_then(|s| s.parse::<i32>().ok());

            // Client-side filtering: only include games that actually match the query
            let query_lower = query.trim().to_lowercase();
            let name_lower = name.to_lowercase();
            
            if name_lower.contains(&query_lower) || query_lower.contains(&name_lower) {
                log::info!("Parsed BGG game: {} (ID: {}, Year: {:?}) - MATCHES QUERY", name, id, year_published);
                
                // Parse the ID as an integer
                let bgg_id_int = id.parse::<i32>()
                    .map_err(|e| anyhow::anyhow!("Invalid BGG ID format: {}", e))?;
                
                // Create a Game object with BGG source
                let game = Game {
                    id: format!("bgg_{}", id), // Use BGG ID as local ID
                    rev: String::new(), // No revision for external games
                    name: name.to_string(),
                    year_published,
                    bgg_id: Some(bgg_id_int),
                    description: None, // We'll get this from details if needed
                    source: shared::models::game::GameSource::BGG,
                };
                
                games.push(game);
            } else {
                log::debug!("Skipping BGG game: {} (ID: {}, Year: {:?}) - DOES NOT MATCH QUERY", name, id, year_published);
            }
        }

        log::info!("BGG API returned {} games for query '{}'", games.len(), query);
        Ok(games)
    }

    pub async fn get_game_details(&self, bgg_id: &str) -> Result<Option<Game>> {
        log::info!("Getting BGG game details for ID: {}", bgg_id);
        
        let thing_url = format!("{}/thing", self.api_url);
        let params = [
            ("id", bgg_id),
            ("stats", "1"),
        ];

        let response = self.build_request(reqwest::Method::GET, &thing_url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("BGG Thing API request failed: {}", response.status()));
        }

        let response_text = response.text().await?;
        log::debug!("BGG thing response: {}", response_text);

        // Parse XML response (simplified - in practice you'd use an XML parser)
        // For now, we'll return None and log that we need XML parsing
        log::warn!("BGG API returns XML, need to implement XML parsing");
        
        Ok(None)
    }

    pub async fn get_popular_games(&self) -> Result<Vec<Game>> {
        log::info!("Getting popular games from BGG API");
        
        let hot_url = format!("{}/hot", self.api_url);
        let params = [
            ("type", "boardgame"),
        ];

        let response = self.build_request(reqwest::Method::GET, &hot_url)
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("BGG Hot API request failed: {}", response.status()));
        }

        let response_text = response.text().await?;
        log::debug!("BGG hot response: {}", response_text);

        // Parse XML response (simplified - in practice you'd use an XML parser)
        // For now, we'll return an empty vector and log that we need XML parsing
        log::warn!("BGG API returns XML, need to implement XML parsing");
        
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test]
    fn test_bgg_service_creation() {
        let service = BGGService::new_with_url("https://boardgamegeek.com/xmlapi2".to_string());
        assert_eq!(service.api_url, "https://boardgamegeek.com/xmlapi2");
    }

    #[test]
    fn test_bgg_service_api_url_format() {
        let service = BGGService::new_with_url("https://boardgamegeek.com/xmlapi2".to_string());
        assert!(service.api_url.starts_with("https://boardgamegeek.com"));
        assert!(service.api_url.contains("xmlapi2"));
    }
} 
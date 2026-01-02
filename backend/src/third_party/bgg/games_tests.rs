#[cfg(test)]
mod bgg_games_tests {
    use super::*;
    use crate::third_party::bgg::games::{BGGService, BGGSearchResponse, BGGSearchItem, BGGId, BGGName, BGGYearPublished, BGGThingResponse, BGGThingItem, BGGThingName};
    use serde_json::json;
    use std::collections::HashMap;

    // Mock HTTP client for testing
    struct MockHttpClient {
        responses: HashMap<String, String>,
        should_fail: bool,
    }

    impl MockHttpClient {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
                should_fail: false,
            }
        }

        fn set_response(&mut self, url: String, response: String) {
            self.responses.insert(url, response);
        }

        fn set_should_fail(&mut self, fail: bool) {
            self.should_fail = fail;
        }
    }

    #[async_trait::async_trait]
    impl reqwest::Client for MockHttpClient {
        async fn get(&self, url: &str) -> Result<reqwest::Response, reqwest::Error> {
            if self.should_fail {
                return Err(reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "Mock connection error"
                )));
            }

            if let Some(response_body) = self.responses.get(url) {
                let response = reqwest::Response::from(
                    http::Response::builder()
                        .status(200)
                        .body(response_body.clone())
                        .unwrap()
                );
                Ok(response)
            } else {
                Err(reqwest::Error::from(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Mock response not found"
                )))
            }
        }

        async fn post(&self, _url: &str) -> Result<reqwest::Response, reqwest::Error> {
            Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "POST not implemented in mock"
            )))
        }

        async fn put(&self, _url: &str) -> Result<reqwest::Response, reqwest::Error> {
            Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "PUT not implemented in mock"
            )))
        }

        async fn delete(&self, _url: &str) -> Result<reqwest::Response, reqwest::Error> {
            Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "DELETE not implemented in mock"
            )))
        }

        async fn head(&self, _url: &str) -> Result<reqwest::Response, reqwest::Error> {
            Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "HEAD not implemented in mock"
            )))
        }

        async fn patch(&self, _url: &str) -> Result<reqwest::Response, reqwest::Error> {
            Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "PATCH not implemented in mock"
            )))
        }

        async fn request(&self, _method: reqwest::Method, _url: &str) -> Result<reqwest::Response, reqwest::Error> {
            Err(reqwest::Error::from(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Generic request not implemented in mock"
            )))
        }
    }

    #[test]
    fn test_bgg_service_creation() {
        let service = BGGService::new();
        assert!(service.base_url.contains("boardgamegeek.com"));
    }

    #[test]
    fn test_bgg_service_clone() {
        let service1 = BGGService::new();
        let service2 = service1.clone();
        
        assert_eq!(service1.base_url, service2.base_url);
    }

    #[test]
    fn test_bgg_service_debug() {
        let service = BGGService::new();
        let debug_str = format!("{:?}", service);
        assert!(debug_str.contains("BGGService"));
        assert!(debug_str.contains("boardgamegeek.com"));
    }

    #[test]
    fn test_bgg_search_response_deserialization() {
        let json = json!({
            "items": [
                {
                    "type": "boardgame",
                    "id": {
                        "value": "12345"
                    },
                    "name": {
                        "value": "Test Game"
                    },
                    "yearpublished": {
                        "value": "2020"
                    }
                }
            ]
        });

        let response: BGGSearchResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].item_type, "boardgame");
        assert_eq!(response.items[0].id.id, "12345");
        assert_eq!(response.items[0].name.name, "Test Game");
        assert_eq!(response.items[0].yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_search_response_empty_items() {
        let json = json!({
            "items": []
        });

        let response: BGGSearchResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.items.len(), 0);
    }

    #[test]
    fn test_bgg_search_response_multiple_items() {
        let json = json!({
            "items": [
                {
                    "type": "boardgame",
                    "id": {
                        "value": "12345"
                    },
                    "name": {
                        "value": "Test Game 1"
                    },
                    "yearpublished": {
                        "value": "2020"
                    }
                },
                {
                    "type": "boardgame",
                    "id": {
                        "value": "67890"
                    },
                    "name": {
                        "value": "Test Game 2"
                    },
                    "yearpublished": {
                        "value": "2021"
                    }
                }
            ]
        });

        let response: BGGSearchResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].name.name, "Test Game 1");
        assert_eq!(response.items[1].name.name, "Test Game 2");
    }

    #[test]
    fn test_bgg_search_item_deserialization() {
        let json = json!({
            "type": "boardgame",
            "id": {
                "value": "12345"
            },
            "name": {
                "value": "Test Game"
            },
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGSearchItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id.id, "12345");
        assert_eq!(item.name.name, "Test Game");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_search_item_missing_year() {
        let json = json!({
            "type": "boardgame",
            "id": {
                "value": "12345"
            },
            "name": {
                "value": "Test Game"
            }
        });

        let item: BGGSearchItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id.id, "12345");
        assert_eq!(item.name.name, "Test Game");
        assert!(item.yearpublished.is_none());
    }

    #[test]
    fn test_bgg_id_deserialization() {
        let json = json!({
            "value": "12345"
        });

        let id: BGGId = serde_json::from_value(json).unwrap();
        
        assert_eq!(id.id, "12345");
    }

    #[test]
    fn test_bgg_id_empty_value() {
        let json = json!({
            "value": ""
        });

        let id: BGGId = serde_json::from_value(json).unwrap();
        
        assert_eq!(id.id, "");
    }

    #[test]
    fn test_bgg_name_deserialization() {
        let json = json!({
            "value": "Test Game"
        });

        let name: BGGName = serde_json::from_value(json).unwrap();
        
        assert_eq!(name.name, "Test Game");
    }

    #[test]
    fn test_bgg_name_empty_value() {
        let json = json!({
            "value": ""
        });

        let name: BGGName = serde_json::from_value(json).unwrap();
        
        assert_eq!(name.name, "");
    }

    #[test]
    fn test_bgg_year_published_deserialization() {
        let json = json!({
            "value": "2020"
        });

        let year: BGGYearPublished = serde_json::from_value(json).unwrap();
        
        assert_eq!(year.year, "2020");
    }

    #[test]
    fn test_bgg_year_published_empty_value() {
        let json = json!({
            "value": ""
        });

        let year: BGGYearPublished = serde_json::from_value(json).unwrap();
        
        assert_eq!(year.year, "");
    }

    #[test]
    fn test_bgg_thing_response_deserialization() {
        let json = json!({
            "items": [
                {
                    "type": "boardgame",
                    "id": "12345",
                    "name": [
                        {
                            "type": "primary",
                            "value": "Test Game"
                        }
                    ],
                    "description": "A test game description",
                    "yearpublished": {
                        "value": "2020"
                    }
                }
            ]
        });

        let response: BGGThingResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].item_type, "boardgame");
        assert_eq!(response.items[0].id, "12345");
        assert_eq!(response.items[0].name.len(), 1);
        assert_eq!(response.items[0].name[0].name_type, "primary");
        assert_eq!(response.items[0].name[0].name_value, "Test Game");
        assert_eq!(response.items[0].description.as_ref().unwrap(), "A test game description");
        assert_eq!(response.items[0].yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_thing_response_empty_items() {
        let json = json!({
            "items": []
        });

        let response: BGGThingResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.items.len(), 0);
    }

    #[test]
    fn test_bgg_thing_item_deserialization() {
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "name": [
                {
                    "type": "primary",
                    "value": "Test Game"
                }
            ],
            "description": "A test game description",
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 1);
        assert_eq!(item.name[0].name_type, "primary");
        assert_eq!(item.name[0].name_value, "Test Game");
        assert_eq!(item.description.as_ref().unwrap(), "A test game description");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_thing_item_missing_description() {
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "name": [
                {
                    "type": "primary",
                    "value": "Test Game"
                }
            ]
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 1);
        assert_eq!(item.name[0].name_type, "primary");
        assert_eq!(item.name[0].name_value, "Test Game");
        assert!(item.description.is_none());
    }

    #[test]
    fn test_bgg_thing_item_multiple_names() {
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "name": [
                {
                    "type": "primary",
                    "value": "Test Game"
                },
                {
                    "type": "alternate",
                    "value": "Alternate Name"
                }
            ]
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 2);
        assert_eq!(item.name[0].name_type, "primary");
        assert_eq!(item.name[0].name_value, "Test Game");
        assert_eq!(item.name[1].name_type, "alternate");
        assert_eq!(item.name[1].name_value, "Alternate Name");
    }

    #[test]
    fn test_bgg_thing_name_deserialization() {
        let json = json!({
            "type": "primary",
            "value": "Test Game"
        });

        let name: BGGThingName = serde_json::from_value(json).unwrap();
        
        assert_eq!(name.name_type, "primary");
        assert_eq!(name.name_value, "Test Game");
    }

    #[test]
    fn test_bgg_thing_name_empty_value() {
        let json = json!({
            "type": "primary",
            "value": ""
        });

        let name: BGGThingName = serde_json::from_value(json).unwrap();
        
        assert_eq!(name.name_type, "primary");
        assert_eq!(name.name_value, "");
    }

    #[test]
    fn test_bgg_thing_name_unicode_value() {
        let json = json!({
            "type": "primary",
            "value": "测试游戏"
        });

        let name: BGGThingName = serde_json::from_value(json).unwrap();
        
        assert_eq!(name.name_type, "primary");
        assert_eq!(name.name_value, "测试游戏");
    }

    #[test]
    fn test_bgg_search_response_serialization() {
        let response = BGGSearchResponse {
            items: vec![
                BGGSearchItem {
                    item_type: "boardgame".to_string(),
                    id: BGGId {
                        id: "12345".to_string(),
                    },
                    name: BGGName {
                        name: "Test Game".to_string(),
                    },
                    yearpublished: Some(BGGYearPublished {
                        year: "2020".to_string(),
                    }),
                }
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("12345"));
        assert!(json.contains("Test Game"));
        assert!(json.contains("2020"));
        assert!(json.contains("boardgame"));
    }

    #[test]
    fn test_bgg_thing_response_serialization() {
        let response = BGGThingResponse {
            items: vec![
                BGGThingItem {
                    item_type: "boardgame".to_string(),
                    id: "12345".to_string(),
                    name: vec![
                        BGGThingName {
                            name_type: "primary".to_string(),
                            name_value: "Test Game".to_string(),
                        }
                    ],
                    description: Some("A test game description".to_string()),
                    yearpublished: Some(BGGYearPublished {
                        year: "2020".to_string(),
                    }),
                }
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("12345"));
        assert!(json.contains("Test Game"));
        assert!(json.contains("A test game description"));
        assert!(json.contains("2020"));
        assert!(json.contains("boardgame"));
    }

    #[test]
    fn test_bgg_service_base_url() {
        let service = BGGService::new();
        assert!(service.base_url.contains("boardgamegeek.com"));
        assert!(service.base_url.starts_with("http"));
    }

    #[test]
    fn test_bgg_service_clone_equality() {
        let service1 = BGGService::new();
        let service2 = service1.clone();
        
        assert_eq!(service1.base_url, service2.base_url);
    }

    #[test]
    fn test_bgg_service_debug_formatting() {
        let service = BGGService::new();
        let debug_str = format!("{:?}", service);
        
        assert!(debug_str.contains("BGGService"));
        assert!(debug_str.contains("boardgamegeek.com"));
    }

    #[test]
    fn test_bgg_search_item_unicode_name() {
        let json = json!({
            "type": "boardgame",
            "id": {
                "value": "12345"
            },
            "name": {
                "value": "测试游戏"
            },
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGSearchItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id.id, "12345");
        assert_eq!(item.name.name, "测试游戏");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_thing_item_unicode_description() {
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "name": [
                {
                    "type": "primary",
                    "value": "Test Game"
                }
            ],
            "description": "这是一个测试游戏描述",
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 1);
        assert_eq!(item.name[0].name_type, "primary");
        assert_eq!(item.name[0].name_value, "Test Game");
        assert_eq!(item.description.as_ref().unwrap(), "这是一个测试游戏描述");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_search_item_special_characters() {
        let json = json!({
            "type": "boardgame",
            "id": {
                "value": "12345"
            },
            "name": {
                "value": "Test Game: Special Edition!"
            },
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGSearchItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id.id, "12345");
        assert_eq!(item.name.name, "Test Game: Special Edition!");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_thing_item_long_description() {
        let long_description = "A".repeat(1000);
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "name": [
                {
                    "type": "primary",
                    "value": "Test Game"
                }
            ],
            "description": long_description,
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 1);
        assert_eq!(item.name[0].name_type, "primary");
        assert_eq!(item.name[0].name_value, "Test Game");
        assert_eq!(item.description.as_ref().unwrap(), long_description);
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_search_item_numeric_id() {
        let json = json!({
            "type": "boardgame",
            "id": {
                "value": "12345"
            },
            "name": {
                "value": "Test Game"
            },
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGSearchItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id.id, "12345");
        assert_eq!(item.name.name, "Test Game");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_search_item_string_id() {
        let json = json!({
            "type": "boardgame",
            "id": {
                "value": "abc123"
            },
            "name": {
                "value": "Test Game"
            },
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGSearchItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id.id, "abc123");
        assert_eq!(item.name.name, "Test Game");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_thing_item_empty_name_array() {
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "name": [],
            "description": "A test game description",
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 0);
        assert_eq!(item.description.as_ref().unwrap(), "A test game description");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }

    #[test]
    fn test_bgg_thing_item_missing_name() {
        let json = json!({
            "type": "boardgame",
            "id": "12345",
            "description": "A test game description",
            "yearpublished": {
                "value": "2020"
            }
        });

        let item: BGGThingItem = serde_json::from_value(json).unwrap();
        
        assert_eq!(item.item_type, "boardgame");
        assert_eq!(item.id, "12345");
        assert_eq!(item.name.len(), 0);
        assert_eq!(item.description.as_ref().unwrap(), "A test game description");
        assert_eq!(item.yearpublished.as_ref().unwrap().year, "2020");
    }
}

#[cfg(test)]
mod google_places_tests {
    use super::*;
    use crate::third_party::google::places::{GooglePlacesService, GoogleAutocompleteResponse, GooglePlaceDetailsResponse, GooglePrediction, GoogleStructuredFormatting, GooglePlaceDetails, GoogleGeometry, GoogleLocation};
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
    fn test_google_places_service_creation() {
        let service = GooglePlacesService::new("test_api_key".to_string());
        assert_eq!(service.api_key, "test_api_key");
    }

    #[test]
    fn test_google_places_service_creation_empty_key() {
        let service = GooglePlacesService::new("".to_string());
        assert_eq!(service.api_key, "");
    }

    #[test]
    fn test_google_places_service_clone() {
        let service1 = GooglePlacesService::new("test_api_key".to_string());
        let service2 = service1.clone();
        
        assert_eq!(service1.api_key, service2.api_key);
    }

    #[test]
    fn test_google_places_service_debug() {
        let service = GooglePlacesService::new("test_api_key".to_string());
        let debug_str = format!("{:?}", service);
        assert!(debug_str.contains("GooglePlacesService"));
        assert!(debug_str.contains("test_api_key"));
    }

    #[test]
    fn test_google_autocomplete_response_deserialization() {
        let json = json!({
            "predictions": [
                {
                    "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
                    "description": "New York, NY, USA",
                    "structured_formatting": {
                        "main_text": "New York",
                        "secondary_text": "NY, USA"
                    }
                }
            ],
            "status": "OK"
        });

        let response: GoogleAutocompleteResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "OK");
        assert_eq!(response.predictions.len(), 1);
        assert_eq!(response.predictions[0].place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(response.predictions[0].description, "New York, NY, USA");
        assert_eq!(response.predictions[0].structured_formatting.main_text, "New York");
        assert_eq!(response.predictions[0].structured_formatting.secondary_text, "NY, USA");
    }

    #[test]
    fn test_google_autocomplete_response_empty_predictions() {
        let json = json!({
            "predictions": [],
            "status": "OK"
        });

        let response: GoogleAutocompleteResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "OK");
        assert_eq!(response.predictions.len(), 0);
    }

    #[test]
    fn test_google_autocomplete_response_error_status() {
        let json = json!({
            "predictions": [],
            "status": "ZERO_RESULTS"
        });

        let response: GoogleAutocompleteResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "ZERO_RESULTS");
        assert_eq!(response.predictions.len(), 0);
    }

    #[test]
    fn test_google_place_details_response_deserialization() {
        let json = json!({
            "result": {
                "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
                "formatted_address": "New York, NY, USA",
                "name": "New York",
                "geometry": {
                    "location": {
                        "lat": 40.7128,
                        "lng": -74.0060
                    }
                }
            },
            "status": "OK"
        });

        let response: GooglePlaceDetailsResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "OK");
        assert_eq!(response.result.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(response.result.formatted_address, "New York, NY, USA");
        assert_eq!(response.result.name, "New York");
        assert_eq!(response.result.geometry.location.lat, 40.7128);
        assert_eq!(response.result.geometry.location.lng, -74.0060);
    }

    #[test]
    fn test_google_place_details_response_error_status() {
        let json = json!({
            "result": {},
            "status": "INVALID_REQUEST"
        });

        let response: GooglePlaceDetailsResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "INVALID_REQUEST");
    }

    #[test]
    fn test_google_prediction_deserialization() {
        let json = json!({
            "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
            "description": "New York, NY, USA",
            "structured_formatting": {
                "main_text": "New York",
                "secondary_text": "NY, USA"
            }
        });

        let prediction: GooglePrediction = serde_json::from_value(json).unwrap();
        
        assert_eq!(prediction.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(prediction.description, "New York, NY, USA");
        assert_eq!(prediction.structured_formatting.main_text, "New York");
        assert_eq!(prediction.structured_formatting.secondary_text, "NY, USA");
    }

    #[test]
    fn test_google_structured_formatting_deserialization() {
        let json = json!({
            "main_text": "New York",
            "secondary_text": "NY, USA"
        });

        let formatting: GoogleStructuredFormatting = serde_json::from_value(json).unwrap();
        
        assert_eq!(formatting.main_text, "New York");
        assert_eq!(formatting.secondary_text, "NY, USA");
    }

    #[test]
    fn test_google_place_details_deserialization() {
        let json = json!({
            "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
            "formatted_address": "New York, NY, USA",
            "name": "New York",
            "geometry": {
                "location": {
                    "lat": 40.7128,
                    "lng": -74.0060
                }
            }
        });

        let details: GooglePlaceDetails = serde_json::from_value(json).unwrap();
        
        assert_eq!(details.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(details.formatted_address, "New York, NY, USA");
        assert_eq!(details.name, "New York");
        assert_eq!(details.geometry.location.lat, 40.7128);
        assert_eq!(details.geometry.location.lng, -74.0060);
    }

    #[test]
    fn test_google_geometry_deserialization() {
        let json = json!({
            "location": {
                "lat": 40.7128,
                "lng": -74.0060
            }
        });

        let geometry: GoogleGeometry = serde_json::from_value(json).unwrap();
        
        assert_eq!(geometry.location.lat, 40.7128);
        assert_eq!(geometry.location.lng, -74.0060);
    }

    #[test]
    fn test_google_location_deserialization() {
        let json = json!({
            "lat": 40.7128,
            "lng": -74.0060
        });

        let location: GoogleLocation = serde_json::from_value(json).unwrap();
        
        assert_eq!(location.lat, 40.7128);
        assert_eq!(location.lng, -74.0060);
    }

    #[test]
    fn test_google_location_negative_coordinates() {
        let json = json!({
            "lat": -33.8688,
            "lng": 151.2093
        });

        let location: GoogleLocation = serde_json::from_value(json).unwrap();
        
        assert_eq!(location.lat, -33.8688);
        assert_eq!(location.lng, 151.2093);
    }

    #[test]
    fn test_google_location_zero_coordinates() {
        let json = json!({
            "lat": 0.0,
            "lng": 0.0
        });

        let location: GoogleLocation = serde_json::from_value(json).unwrap();
        
        assert_eq!(location.lat, 0.0);
        assert_eq!(location.lng, 0.0);
    }

    #[test]
    fn test_google_location_extreme_coordinates() {
        let json = json!({
            "lat": 90.0,
            "lng": 180.0
        });

        let location: GoogleLocation = serde_json::from_value(json).unwrap();
        
        assert_eq!(location.lat, 90.0);
        assert_eq!(location.lng, 180.0);
    }

    #[test]
    fn test_google_prediction_empty_description() {
        let json = json!({
            "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
            "description": "",
            "structured_formatting": {
                "main_text": "",
                "secondary_text": ""
            }
        });

        let prediction: GooglePrediction = serde_json::from_value(json).unwrap();
        
        assert_eq!(prediction.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(prediction.description, "");
        assert_eq!(prediction.structured_formatting.main_text, "");
        assert_eq!(prediction.structured_formatting.secondary_text, "");
    }

    #[test]
    fn test_google_place_details_empty_fields() {
        let json = json!({
            "place_id": "",
            "formatted_address": "",
            "name": "",
            "geometry": {
                "location": {
                    "lat": 0.0,
                    "lng": 0.0
                }
            }
        });

        let details: GooglePlaceDetails = serde_json::from_value(json).unwrap();
        
        assert_eq!(details.place_id, "");
        assert_eq!(details.formatted_address, "");
        assert_eq!(details.name, "");
        assert_eq!(details.geometry.location.lat, 0.0);
        assert_eq!(details.geometry.location.lng, 0.0);
    }

    #[test]
    fn test_google_autocomplete_response_multiple_predictions() {
        let json = json!({
            "predictions": [
                {
                    "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
                    "description": "New York, NY, USA",
                    "structured_formatting": {
                        "main_text": "New York",
                        "secondary_text": "NY, USA"
                    }
                },
                {
                    "place_id": "ChIJN1t_tDeuEmsRUsoyG83frY4",
                    "description": "New York, NY, USA",
                    "structured_formatting": {
                        "main_text": "New York",
                        "secondary_text": "NY, USA"
                    }
                }
            ],
            "status": "OK"
        });

        let response: GoogleAutocompleteResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "OK");
        assert_eq!(response.predictions.len(), 2);
        assert_eq!(response.predictions[0].place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(response.predictions[1].place_id, "ChIJN1t_tDeuEmsRUsoyG83frY4");
    }

    #[test]
    fn test_google_place_details_response_missing_fields() {
        let json = json!({
            "result": {
                "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
                "geometry": {
                    "location": {
                        "lat": 40.7128,
                        "lng": -74.0060
                    }
                }
            },
            "status": "OK"
        });

        let response: GooglePlaceDetailsResponse = serde_json::from_value(json).unwrap();
        
        assert_eq!(response.status, "OK");
        assert_eq!(response.result.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(response.result.formatted_address, "");
        assert_eq!(response.result.name, "");
        assert_eq!(response.result.geometry.location.lat, 40.7128);
        assert_eq!(response.result.geometry.location.lng, -74.0060);
    }

    #[test]
    fn test_google_structured_formatting_missing_fields() {
        let json = json!({
            "main_text": "New York"
        });

        let formatting: GoogleStructuredFormatting = serde_json::from_value(json).unwrap();
        
        assert_eq!(formatting.main_text, "New York");
        assert_eq!(formatting.secondary_text, "");
    }

    #[test]
    fn test_google_geometry_missing_location() {
        let json = json!({});

        let geometry: GoogleGeometry = serde_json::from_value(json).unwrap();
        
        assert_eq!(geometry.location.lat, 0.0);
        assert_eq!(geometry.location.lng, 0.0);
    }

    #[test]
    fn test_google_location_missing_coordinates() {
        let json = json!({});

        let location: GoogleLocation = serde_json::from_value(json).unwrap();
        
        assert_eq!(location.lat, 0.0);
        assert_eq!(location.lng, 0.0);
    }

    #[test]
    fn test_google_prediction_unicode_description() {
        let json = json!({
            "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
            "description": "北京市, 中国",
            "structured_formatting": {
                "main_text": "北京市",
                "secondary_text": "中国"
            }
        });

        let prediction: GooglePrediction = serde_json::from_value(json).unwrap();
        
        assert_eq!(prediction.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(prediction.description, "北京市, 中国");
        assert_eq!(prediction.structured_formatting.main_text, "北京市");
        assert_eq!(prediction.structured_formatting.secondary_text, "中国");
    }

    #[test]
    fn test_google_place_details_unicode_name() {
        let json = json!({
            "place_id": "ChIJd8BlQ2BZwokRAFQEcDlJRAI",
            "formatted_address": "北京市, 中国",
            "name": "北京市",
            "geometry": {
                "location": {
                    "lat": 39.9042,
                    "lng": 116.4074
                }
            }
        });

        let details: GooglePlaceDetails = serde_json::from_value(json).unwrap();
        
        assert_eq!(details.place_id, "ChIJd8BlQ2BZwokRAFQEcDlJRAI");
        assert_eq!(details.formatted_address, "北京市, 中国");
        assert_eq!(details.name, "北京市");
        assert_eq!(details.geometry.location.lat, 39.9042);
        assert_eq!(details.geometry.location.lng, 116.4074);
    }

    #[test]
    fn test_google_autocomplete_response_serialization() {
        let response = GoogleAutocompleteResponse {
            predictions: vec![
                GooglePrediction {
                    place_id: "ChIJd8BlQ2BZwokRAFQEcDlJRAI".to_string(),
                    description: "New York, NY, USA".to_string(),
                    structured_formatting: GoogleStructuredFormatting {
                        main_text: "New York".to_string(),
                        secondary_text: "NY, USA".to_string(),
                    },
                }
            ],
            status: "OK".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ChIJd8BlQ2BZwokRAFQEcDlJRAI"));
        assert!(json.contains("New York, NY, USA"));
        assert!(json.contains("OK"));
    }

    #[test]
    fn test_google_place_details_response_serialization() {
        let response = GooglePlaceDetailsResponse {
            result: GooglePlaceDetails {
                place_id: "ChIJd8BlQ2BZwokRAFQEcDlJRAI".to_string(),
                formatted_address: "New York, NY, USA".to_string(),
                name: "New York".to_string(),
                geometry: GoogleGeometry {
                    location: GoogleLocation {
                        lat: 40.7128,
                        lng: -74.0060,
                    },
                },
            },
            status: "OK".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ChIJd8BlQ2BZwokRAFQEcDlJRAI"));
        assert!(json.contains("New York, NY, USA"));
        assert!(json.contains("40.7128"));
        assert!(json.contains("-74.0060"));
        assert!(json.contains("OK"));
    }

    #[test]
    fn test_google_places_service_debug_formatting() {
        let service = GooglePlacesService::new("test_api_key".to_string());
        let debug_str = format!("{:?}", service);
        
        // Should contain the service name and API key
        assert!(debug_str.contains("GooglePlacesService"));
        assert!(debug_str.contains("test_api_key"));
    }

    #[test]
    fn test_google_places_service_clone_equality() {
        let service1 = GooglePlacesService::new("test_api_key".to_string());
        let service2 = service1.clone();
        
        // Both should have the same API key
        assert_eq!(service1.api_key, service2.api_key);
        
        // They should be equal
        assert_eq!(service1.api_key, service2.api_key);
    }

    #[test]
    fn test_google_places_service_empty_api_key() {
        let service = GooglePlacesService::new("".to_string());
        assert_eq!(service.api_key, "");
    }

    #[test]
    fn test_google_places_service_long_api_key() {
        let long_key = "a".repeat(1000);
        let service = GooglePlacesService::new(long_key.clone());
        assert_eq!(service.api_key, long_key);
    }

    #[test]
    fn test_google_places_service_special_characters_api_key() {
        let special_key = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let service = GooglePlacesService::new(special_key.to_string());
        assert_eq!(service.api_key, special_key);
    }

    #[test]
    fn test_google_places_service_unicode_api_key() {
        let unicode_key = "测试API密钥";
        let service = GooglePlacesService::new(unicode_key.to_string());
        assert_eq!(service.api_key, unicode_key);
    }
}

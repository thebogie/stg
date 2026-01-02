use gloo_net::http::Request;
use gloo_storage::Storage;


/// Creates a request with Authorization header from localStorage
pub fn authenticated_request(method: &str, url: &str) -> gloo_net::http::RequestBuilder {
    let mut req = match method.to_uppercase().as_str() {
        "GET" => Request::get(url),
        "POST" => Request::post(url),
        "PUT" => Request::put(url),
        "DELETE" => Request::delete(url),
        "PATCH" => Request::patch(url),
        _ => Request::get(url), // Default to GET
    };

    // Attach Authorization header for all authenticated requests
    if let Ok(session_id) = gloo_storage::LocalStorage::get::<String>("session_id") {
        req = req.header("Authorization", &format!("Bearer {}", session_id));
    } else {
        // No session_id found, continue without authentication
    }

    req
}

/// Creates a GET request with authentication
pub fn authenticated_get(url: &str) -> gloo_net::http::RequestBuilder {
    authenticated_request("GET", url)
}

/// Creates a POST request with authentication
pub fn authenticated_post(url: &str) -> gloo_net::http::RequestBuilder {
    authenticated_request("POST", url)
}

/// Creates a PUT request with authentication
pub fn authenticated_put(url: &str) -> gloo_net::http::RequestBuilder {
    authenticated_request("PUT", url)
}

/// Creates a DELETE request with authentication
pub fn authenticated_delete(url: &str) -> gloo_net::http::RequestBuilder {
    authenticated_request("DELETE", url)
}

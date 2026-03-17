//! HTTP client for the AgentGIF API.

use reqwest::blocking::{Client as HttpClient, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

pub const DEFAULT_BASE_URL: &str = "https://agentgif.com";

#[derive(Debug)]
pub struct ApiError {
    pub message: String,
    pub status: u16,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API error {}: {}", self.status, self.message)
    }
}

impl std::error::Error for ApiError {}

pub struct Client {
    base_url: String,
    api_key: String,
    http: HttpClient,
}

impl Client {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            api_key: crate::config::get_api_key(),
            http: HttpClient::new(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: String::new(),
            http: HttpClient::new(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_base_url_and_key(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            http: HttpClient::new(),
        }
    }

    fn request(&self, method: &str, path: &str) -> reqwest::blocking::RequestBuilder {
        let url = format!("{}/api/v1{}", self.base_url, path);
        let builder = match method {
            "GET" => self.http.get(&url),
            "POST" => self.http.post(&url),
            "PATCH" => self.http.patch(&url),
            "DELETE" => self.http.delete(&url),
            _ => self.http.get(&url),
        };
        let builder = builder.header("Content-Type", "application/json");
        if !self.api_key.is_empty() {
            builder.header("Authorization", format!("Token {}", self.api_key))
        } else {
            builder
        }
    }

    fn handle_response(&self, resp: Response) -> Result<Value, Box<dyn std::error::Error>> {
        let status = resp.status().as_u16();
        let body = resp.text()?;
        if status >= 400 {
            let msg = if let Ok(obj) = serde_json::from_str::<Value>(&body) {
                obj["error"]
                    .as_str()
                    .or_else(|| obj["detail"].as_str())
                    .unwrap_or(&body)
                    .to_string()
            } else {
                body
            };
            return Err(Box::new(ApiError {
                message: msg,
                status,
            }));
        }
        Ok(serde_json::from_str(&body).unwrap_or(Value::Null))
    }

    pub fn whoami(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request("GET", "/users/me/").send()?;
        self.handle_response(resp)
    }

    pub fn search(&self, query: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let encoded = urlencoding(query);
        let resp = self
            .request("GET", &format!("/search/?q={encoded}"))
            .send()?;
        self.handle_response(resp)
    }

    pub fn list_gifs(&self, repo: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let path = if repo.is_empty() {
            "/gifs/me/".to_string()
        } else {
            format!("/gifs/me/?repo={}", urlencoding(repo))
        };
        let resp = self.request("GET", &path).send()?;
        self.handle_response(resp)
    }

    pub fn get_gif(&self, gif_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request("GET", &format!("/gifs/{gif_id}/")).send()?;
        self.handle_response(resp)
    }

    pub fn embed_codes(
        &self,
        gif_id: &str,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let data = self.get_gif(gif_id)?;
        let mut codes = HashMap::new();
        if let Some(embed) = data.get("embed").and_then(|v| v.as_object()) {
            for (k, v) in embed {
                if let Some(s) = v.as_str() {
                    codes.insert(k.clone(), s.to_string());
                }
            }
        }
        Ok(codes)
    }

    pub fn update_gif(
        &self,
        gif_id: &str,
        fields: &HashMap<String, String>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self
            .request("PATCH", &format!("/gifs/{gif_id}/"))
            .json(fields)
            .send()?;
        self.handle_response(resp)
    }

    pub fn delete_gif(&self, gif_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let resp = self
            .request("DELETE", &format!("/gifs/{gif_id}/"))
            .send()?;
        if resp.status().as_u16() >= 400 {
            let body = resp.text()?;
            return Err(Box::new(ApiError {
                message: body,
                status: 400,
            }));
        }
        Ok(())
    }

    pub fn upload(
        &self,
        gif_path: &str,
        opts: &HashMap<String, String>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let file_bytes = fs::read(gif_path)?;
        let file_name = Path::new(gif_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut form = reqwest::blocking::multipart::Form::new().part(
            "gif",
            reqwest::blocking::multipart::Part::bytes(file_bytes)
                .file_name(file_name)
                .mime_str("image/gif")?,
        );

        for (k, v) in opts {
            if !v.is_empty() && k != "cast_path" {
                form = form.text(k.clone(), v.clone());
            }
        }

        if let Some(cast_path) = opts.get("cast_path") {
            if !cast_path.is_empty() {
                if let Ok(cast_bytes) = fs::read(cast_path) {
                    let cast_name = Path::new(cast_path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    form = form.part(
                        "cast",
                        reqwest::blocking::multipart::Part::bytes(cast_bytes)
                            .file_name(cast_name),
                    );
                }
            }
        }

        let url = format!("{}/api/v1/gifs/", self.base_url);
        let mut builder = self.http.post(&url).multipart(form);
        if !self.api_key.is_empty() {
            builder = builder.header("Authorization", format!("Token {}", self.api_key));
        }
        let resp = builder.send()?;
        self.handle_response(resp)
    }

    pub fn badge_url(
        &self,
        provider: &str,
        package: &str,
        opts: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut params = vec![
            format!("provider={}", urlencoding(provider)),
            format!("package={}", urlencoding(package)),
        ];
        for (k, v) in opts {
            if !v.is_empty() {
                params.push(format!("{}={}", urlencoding(k), urlencoding(v)));
            }
        }
        let query = params.join("&");
        let resp = self
            .request("GET", &format!("/badge-url/?{query}"))
            .send()?;
        let status = resp.status().as_u16();
        let body = resp.text()?;
        if status >= 400 {
            return Err(Box::new(ApiError {
                message: body,
                status,
            }));
        }
        Ok(serde_json::from_str(&body)?)
    }

    pub fn badge_themes(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request("GET", "/themes/badges/").send()?;
        self.handle_response(resp)
    }

    pub fn cli_version(&self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let resp = self.request("GET", "/cli/version/").send()?;
        let status = resp.status().as_u16();
        let body = resp.text()?;
        if status >= 400 {
            return Err(Box::new(ApiError {
                message: body,
                status,
            }));
        }
        Ok(serde_json::from_str(&body)?)
    }

    pub fn generate_tape(&self, payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self
            .request("POST", "/gifs/generate/")
            .json(payload)
            .send()?;
        self.handle_response(resp)
    }

    pub fn generate_status(&self, job_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self
            .request("GET", &format!("/gifs/generate/{job_id}/"))
            .send()?;
        self.handle_response(resp)
    }

    pub fn device_auth(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request("POST", "/auth/device/").body("{}").send()?;
        self.handle_response(resp)
    }

    pub fn device_token(
        &self,
        device_code: &str,
    ) -> Result<(Value, u16), Box<dyn std::error::Error>> {
        let url = format!("{}/api/v1/auth/device/token/", self.base_url);
        let body = format!(r#"{{"device_code":"{}"}}"#, device_code);
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()?;
        let status = resp.status().as_u16();
        let text = resp.text()?;
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        Ok((value, status))
    }
}

fn urlencoding(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let err = ApiError {
            message: "Not found".into(),
            status: 404,
        };
        assert_eq!(err.to_string(), "API error 404: Not found");
    }

    #[test]
    fn test_default_base_url() {
        assert_eq!(DEFAULT_BASE_URL, "https://agentgif.com");
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert_eq!(urlencoding("@fyipedia/colorfyi"), "%40fyipedia%2Fcolorfyi");
        assert_eq!(urlencoding("simple"), "simple");
    }

    #[test]
    fn test_urlencoding_special_chars() {
        assert_eq!(urlencoding("a+b"), "a%2Bb");
        assert_eq!(urlencoding("a&b=c"), "a%26b%3Dc");
    }
}

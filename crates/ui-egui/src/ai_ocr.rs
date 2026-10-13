//! Multi-provider AI Vision OCR client for ultra-accurate text extraction.
//!
//! Supports Google Gemini, OpenAI, Anthropic Claude, and Custom OpenAI-compatible endpoints.

use serde::{Deserialize, Serialize};
use pdfcraft_engine::ocr::PlacedWord;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProvider {
    Gemini,
    OpenAi,
    Anthropic,
    Custom,
}

impl AiProvider {
    pub const ALL: [AiProvider; 4] = [
        AiProvider::Gemini,
        AiProvider::OpenAi,
        AiProvider::Anthropic,
        AiProvider::Custom,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::Gemini => "Google Gemini",
            AiProvider::OpenAi => "OpenAI",
            AiProvider::Anthropic => "Anthropic Claude",
            AiProvider::Custom => "Custom (OpenAI-Compatible)",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Gemini => "gemini-2.5-flash",
            AiProvider::OpenAi => "gpt-4o-mini",
            AiProvider::Anthropic => "claude-3-5-haiku-20241022",
            AiProvider::Custom => "default",
        }
    }

    pub fn presets(&self) -> &'static [&'static str] {
        match self {
            AiProvider::Gemini => &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-1.5-flash", "gemini-1.5-pro"],
            AiProvider::OpenAi => &["gpt-4o-mini", "gpt-4o", "chatgpt-4o-latest"],
            AiProvider::Anthropic => &["claude-3-5-haiku-20241022", "claude-3-5-sonnet-20241022"],
            AiProvider::Custom => &["default"],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: AiProvider,
    pub api_key: String,
    pub model: String,
    pub custom_endpoint: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: AiProvider::Gemini,
            api_key: String::new(),
            model: "gemini-2.5-flash".into(),
            custom_endpoint: "http://localhost:11434/v1/chat/completions".into(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn http_agent() -> Result<ureq::Agent, String> {
    let roots = rustls_native_certs::load_native_certs();
    let certs: Vec<ureq::tls::Certificate<'static>> =
        roots.certs.iter().map(|c| ureq::tls::Certificate::from_der(c.as_ref()).to_owned()).collect();
    if certs.is_empty() {
        return Err("No trusted system certificates found".into());
    }
    let tls_config = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::new_with_certs(&certs))
        .build();
    Ok(ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(45)))
        .tls_config(tls_config)
        .build()
        .new_agent())
}

/// Test connection to the configured AI Provider.
pub fn test_connection(config: &AiConfig) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = config;
        return Err("AI API connection is supported on desktop builds".into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if config.api_key.trim().is_empty() && config.provider != AiProvider::Custom {
            return Err("Please enter an API Key".into());
        }

        let agent = http_agent()?;
        match config.provider {
            AiProvider::Gemini => {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    config.model.trim(),
                    config.api_key.trim()
                );
                let payload = serde_json::json!({
                    "contents": [{
                        "parts": [{"text": "Reply with 'OK'"}]
                    }]
                });
                let mut resp = agent
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .send(payload.to_string().as_bytes())
                    .map_err(|e| format!("Gemini connection failed: {e}"))?;
                if resp.status().as_u16() != 200 {
                    let err = resp.body_mut().with_config().limit(4096).read_to_string().unwrap_or_default();
                    return Err(format!("Gemini HTTP {}: {err}", resp.status().as_u16()));
                }
                Ok(format!("Connected to Google Gemini ({}) successfully!", config.model))
            }
            AiProvider::OpenAi => {
                let url = "https://api.openai.com/v1/chat/completions";
                let payload = serde_json::json!({
                    "model": config.model.trim(),
                    "messages": [{"role": "user", "content": "Reply with 'OK'"}],
                    "max_tokens": 5
                });
                let mut resp = agent
                    .post(url)
                    .header("Content-Type", "application/json")
                    .header("Authorization", &format!("Bearer {}", config.api_key.trim()))
                    .send(payload.to_string().as_bytes())
                    .map_err(|e| format!("OpenAI connection failed: {e}"))?;
                if resp.status().as_u16() != 200 {
                    let err = resp.body_mut().with_config().limit(4096).read_to_string().unwrap_or_default();
                    return Err(format!("OpenAI HTTP {}: {err}", resp.status().as_u16()));
                }
                Ok(format!("Connected to OpenAI ({}) successfully!", config.model))
            }
            AiProvider::Anthropic => {
                let url = "https://api.anthropic.com/v1/messages";
                let payload = serde_json::json!({
                    "model": config.model.trim(),
                    "max_tokens": 5,
                    "messages": [{"role": "user", "content": "Reply with 'OK'"}]
                });
                let mut resp = agent
                    .post(url)
                    .header("Content-Type", "application/json")
                    .header("x-api-key", config.api_key.trim())
                    .header("anthropic-version", "2023-06-01")
                    .send(payload.to_string().as_bytes())
                    .map_err(|e| format!("Anthropic connection failed: {e}"))?;
                if resp.status().as_u16() != 200 {
                    let err = resp.body_mut().with_config().limit(4096).read_to_string().unwrap_or_default();
                    return Err(format!("Anthropic HTTP {}: {err}", resp.status().as_u16()));
                }
                Ok(format!("Connected to Anthropic Claude ({}) successfully!", config.model))
            }
            AiProvider::Custom => {
                let url = config.custom_endpoint.trim();
                let payload = serde_json::json!({
                    "model": config.model.trim(),
                    "messages": [{"role": "user", "content": "Reply with 'OK'"}],
                    "max_tokens": 5
                });
                let mut req = agent.post(url).header("Content-Type", "application/json");
                if !config.api_key.trim().is_empty() {
                    req = req.header("Authorization", &format!("Bearer {}", config.api_key.trim()));
                }
                let mut resp = req
                    .send(payload.to_string().as_bytes())
                    .map_err(|e| format!("Custom endpoint connection failed: {e}"))?;
                if resp.status().as_u16() != 200 {
                    let err = resp.body_mut().with_config().limit(4096).read_to_string().unwrap_or_default();
                    return Err(format!("Custom endpoint HTTP {}: {err}", resp.status().as_u16()));
                }
                Ok("Connected to Custom AI endpoint successfully!".into())
            }
        }
    }
}

/// Convert RGBA pixels to PNG bytes in memory.
pub fn encode_rgba_to_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    drop(writer);
    Ok(out)
}

/// Perform AI Vision OCR on a rendered page image.
///
/// Returns words with exact bounding coordinates mapped to PDF user space.
pub fn recognize_page_with_ai(
    config: &AiConfig,
    rgba: &[u8],
    width: usize,
    height: usize,
    page_width: f64,
    page_height: f64,
) -> Result<Vec<PlacedWord>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (config, rgba, width, height, page_width, page_height);
        return Err("AI Vision OCR requires the desktop app".into());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use base64::Engine;
        let png_bytes = encode_rgba_to_png(rgba, width as u32, height as u32)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        let agent = http_agent()?;

        let system_prompt = "You are a high-precision OCR and document transcription engine. \
Transcribe all text in this document image with 100% exact fidelity in both Thai and English. \
Do not miss any words, tone marks, or diacritics. \
Return ONLY a valid JSON array of objects, where each object has: \
\"box\": [ymin, xmin, ymax, xmax] (normalized integers from 0 to 1000, where 0,0 is top-left and 1000,1000 is bottom-right) \
and \"text\": \"word or line segment\". \
Do not wrap in markdown code fences, return pure JSON.";

        let json_text = match config.provider {
            AiProvider::Gemini => {
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    config.model.trim(),
                    config.api_key.trim()
                );
                let payload = serde_json::json!({
                    "contents": [{
                        "parts": [
                            {"text": system_prompt},
                            {
                                "inline_data": {
                                    "mime_type": "image/png",
                                    "data": b64
                                }
                            }
                        ]
                    }],
                    "generationConfig": {
                        "responseMimeType": "application/json"
                    }
                });
                let mut resp = agent
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .send(payload.to_string().as_bytes())
                    .map_err(|e| format!("Gemini Vision OCR failed: {e}"))?;
                let body: serde_json::Value = serde_json::from_reader(resp.body_mut().as_reader())
                    .map_err(|e| format!("Gemini invalid JSON response: {e}"))?;
                body["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            }
            AiProvider::OpenAi | AiProvider::Custom => {
                let url = if config.provider == AiProvider::Custom {
                    config.custom_endpoint.trim()
                } else {
                    "https://api.openai.com/v1/chat/completions"
                };
                let payload = serde_json::json!({
                    "model": config.model.trim(),
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "text", "text": system_prompt},
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:image/png;base64,{b64}")
                                }
                            }
                        ]
                    }],
                    "max_tokens": 4096
                });
                let mut req = agent.post(url).header("Content-Type", "application/json");
                if !config.api_key.trim().is_empty() {
                    req = req.header("Authorization", &format!("Bearer {}", config.api_key.trim()));
                }
                let mut resp = req.send(payload.to_string().as_bytes()).map_err(|e| format!("AI Vision request failed: {e}"))?;
                let body: serde_json::Value = serde_json::from_reader(resp.body_mut().as_reader())
                    .map_err(|e| format!("Invalid JSON response: {e}"))?;
                body["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            }
            AiProvider::Anthropic => {
                let url = "https://api.anthropic.com/v1/messages";
                let payload = serde_json::json!({
                    "model": config.model.trim(),
                    "max_tokens": 4096,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": "image/png",
                                    "data": b64
                                }
                            },
                            {"type": "text", "text": system_prompt}
                        ]
                    }]
                });
                let mut resp = agent
                    .post(url)
                    .header("Content-Type", "application/json")
                    .header("x-api-key", config.api_key.trim())
                    .header("anthropic-version", "2023-06-01")
                    .send(payload.to_string().as_bytes())
                    .map_err(|e| format!("Claude Vision OCR failed: {e}"))?;
                let body: serde_json::Value = serde_json::from_reader(resp.body_mut().as_reader())
                    .map_err(|e| format!("Claude invalid JSON response: {e}"))?;
                body["content"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            }
        };

        parse_ocr_response(&json_text, page_width, page_height)
    }
}

/// Parses the AI's OCR JSON response into PlacedWords in PDF user space.
pub fn parse_ocr_response(json_text: &str, page_width: f64, page_height: f64) -> Result<Vec<PlacedWord>, String> {
    let clean = json_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = serde_json::from_str(clean).map_err(|e| format!("Failed to parse OCR JSON: {e}"))?;

    let items = parsed.as_array().or_else(|| parsed.get("items").and_then(|v| v.as_array())).ok_or_else(|| {
        "Expected JSON array of OCR items".to_string()
    })?;

    let mut words = Vec::new();

    for item in items {
        let text = item["text"].as_str().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }

        // Box: [ymin, xmin, ymax, xmax] in 0..1000
        let coords = item.get("box")
            .or_else(|| item.get("box_2d"))
            .or_else(|| item.get("rect"))
            .and_then(|v| v.as_array());

        let (ymin, xmin, ymax, xmax) = if let Some(arr) = coords {
            if arr.len() == 4 {
                (
                    arr[0].as_f64().unwrap_or(0.0),
                    arr[1].as_f64().unwrap_or(0.0),
                    arr[2].as_f64().unwrap_or(100.0),
                    arr[3].as_f64().unwrap_or(100.0),
                )
            } else {
                continue;
            }
        } else {
            continue;
        };

        // Convert normalized [0..1000] (origin top-left) to PDF user space (origin bottom-left):
        let left = (xmin.min(xmax) / 1000.0) * page_width;
        let right = (xmin.max(xmax) / 1000.0) * page_width;
        let bottom = page_height - (ymin.max(ymax) / 1000.0) * page_height;
        let top = page_height - (ymin.min(ymax) / 1000.0) * page_height;

        let width = (right - left).max(1.0);
        let height = (top - bottom).max(1.0);

        // If the segment has spaces, split into separate PlacedWords along the horizontal span
        let sub_words: Vec<&str> = text.split_whitespace().collect();
        if sub_words.len() > 1 {
            let total_chars: usize = sub_words.iter().map(|w| w.chars().count()).sum();
            let mut curr_x = left;
            for sw in sub_words {
                let char_ratio = sw.chars().count() as f64 / total_chars.max(1) as f64;
                let word_w = width * char_ratio;
                words.push(PlacedWord {
                    text: sw.to_string(),
                    origin: [curr_x, bottom],
                    across: [word_w, 0.0],
                    up: [0.0, height],
                });
                curr_x += word_w;
            }
        } else {
            words.push(PlacedWord {
                text: text.to_string(),
                origin: [left, bottom],
                across: [width, 0.0],
                up: [0.0, height],
            });
        }
    }

    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_ocr_boxes() {
        let json = r#"[
            {"box": [100, 200, 150, 400], "text": "ทดสอบระบบภาษาไทย"},
            {"box": [200, 100, 240, 500], "text": "PdfCraft OCR Engine"}
        ]"#;
        let words = parse_ocr_response(json, 600.0, 800.0).unwrap();
        assert_eq!(words.len(), 4); // "ทดสอบระบบภาษาไทย" + 3 english words
    }
}

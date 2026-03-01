//! TTS (Text-to-Speech) integration — OpenAI, Edge TTS, ElevenLabs.
//!
//! Converts agent responses to audio for voice assistants like Xiaozhi.

use serde::{Deserialize, Serialize};

/// TTS provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// TTS provider: "openai", "edge", "elevenlabs"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Voice name/ID
    #[serde(default = "default_voice")]
    pub voice: String,
    /// API key (for OpenAI/ElevenLabs)
    #[serde(default)]
    pub api_key: String,
    /// Speed multiplier (0.5 - 2.0)
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Output format
    #[serde(default = "default_format")]
    pub format: String,
    /// Whether TTS is enabled
    #[serde(default)]
    pub enabled: bool,
}

fn default_provider() -> String { "edge".to_string() }
fn default_voice() -> String { "vi-VN-HoaiMyNeural".to_string() }
fn default_speed() -> f32 { 1.0 }
fn default_format() -> String { "mp3".to_string() }

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            voice: default_voice(),
            api_key: String::new(),
            speed: 1.0,
            format: default_format(),
            enabled: false,
        }
    }
}

/// TTS synthesis result.
#[derive(Debug)]
pub struct TtsResult {
    /// Audio data bytes.
    pub audio: Vec<u8>,
    /// Content type (e.g., "audio/mpeg").
    pub content_type: String,
    /// Duration estimate in seconds.
    pub duration_secs: f32,
}

/// TTS engine — synthesizes speech from text.
pub struct TtsEngine {
    config: TtsConfig,
    client: reqwest::Client,
}

impl TtsEngine {
    /// Create a new TTS engine.
    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Synthesize text to speech.
    pub async fn synthesize(&self, text: &str) -> Result<TtsResult, String> {
        match self.config.provider.as_str() {
            "openai" => self.openai_tts(text).await,
            "elevenlabs" => self.elevenlabs_tts(text).await,
            "edge" | _ => self.edge_tts(text).await,
        }
    }

    /// OpenAI TTS API (tts-1, tts-1-hd).
    async fn openai_tts(&self, text: &str) -> Result<TtsResult, String> {
        let body = serde_json::json!({
            "model": "tts-1",
            "input": text,
            "voice": self.config.voice,
            "speed": self.config.speed,
            "response_format": self.config.format,
        });

        let response: reqwest::Response = self.client
            .post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI TTS error: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let err: String = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI TTS {status}: {}", &err[..err.len().min(200)]));
        }

        let audio: Vec<u8> = response.bytes().await.map_err(|e| format!("Read error: {e}"))?.to_vec();
        let duration_secs = text.len() as f32 / 15.0; // Rough estimate

        Ok(TtsResult {
            audio,
            content_type: format!("audio/{}", self.config.format),
            duration_secs,
        })
    }

    /// ElevenLabs TTS API.
    async fn elevenlabs_tts(&self, text: &str) -> Result<TtsResult, String> {
        let voice_id = &self.config.voice;
        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");

        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.75,
            }
        });

        let response: reqwest::Response = self.client
            .post(&url)
            .header("xi-api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("ElevenLabs TTS error: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let err: String = response.text().await.unwrap_or_default();
            return Err(format!("ElevenLabs TTS {status}: {}", &err[..err.len().min(200)]));
        }

        let audio: Vec<u8> = response.bytes().await.map_err(|e| format!("Read error: {e}"))?.to_vec();
        let duration_secs = text.len() as f32 / 15.0;

        Ok(TtsResult {
            audio,
            content_type: "audio/mpeg".to_string(),
            duration_secs,
        })
    }

    /// Edge TTS — free Microsoft Edge text-to-speech (via local CLI).
    /// Falls back to a simple placeholder if edge-tts is not installed.
    async fn edge_tts(&self, text: &str) -> Result<TtsResult, String> {
        // Try edge-tts CLI (pip install edge-tts)
        let output_path = format!("/tmp/bizclaw_tts_{}.mp3", uuid::Uuid::new_v4());
        let safe_text = text.replace('"', r#"\""#);

        let result = tokio::process::Command::new("edge-tts")
            .args([
                "--voice", &self.config.voice,
                "--rate", &format!("{:+}%", ((self.config.speed - 1.0) * 100.0) as i32),
                "--text", &safe_text,
                "--write-media", &output_path,
            ])
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let audio = tokio::fs::read(&output_path).await
                    .map_err(|e| format!("Read TTS output error: {e}"))?;
                let _ = tokio::fs::remove_file(&output_path).await;
                let duration_secs = text.len() as f32 / 15.0;

                Ok(TtsResult {
                    audio,
                    content_type: "audio/mpeg".to_string(),
                    duration_secs,
                })
            }
            _ => {
                tracing::warn!("edge-tts not available, returning empty audio");
                Err("edge-tts not installed. Run: pip install edge-tts".to_string())
            }
        }
    }

    /// List available voices for current provider.
    pub fn available_voices(&self) -> Vec<VoiceInfo> {
        match self.config.provider.as_str() {
            "openai" => vec![
                VoiceInfo { id: "alloy".into(), name: "Alloy".into(), lang: "en".into() },
                VoiceInfo { id: "echo".into(), name: "Echo".into(), lang: "en".into() },
                VoiceInfo { id: "fable".into(), name: "Fable".into(), lang: "en".into() },
                VoiceInfo { id: "onyx".into(), name: "Onyx".into(), lang: "en".into() },
                VoiceInfo { id: "nova".into(), name: "Nova".into(), lang: "en".into() },
                VoiceInfo { id: "shimmer".into(), name: "Shimmer".into(), lang: "en".into() },
            ],
            "edge" | _ => vec![
                VoiceInfo { id: "vi-VN-HoaiMyNeural".into(), name: "Hoài My (VN)".into(), lang: "vi".into() },
                VoiceInfo { id: "vi-VN-NamMinhNeural".into(), name: "Nam Minh (VN)".into(), lang: "vi".into() },
                VoiceInfo { id: "en-US-AriaNeural".into(), name: "Aria (US)".into(), lang: "en".into() },
                VoiceInfo { id: "en-US-GuyNeural".into(), name: "Guy (US)".into(), lang: "en".into() },
                VoiceInfo { id: "ja-JP-NanamiNeural".into(), name: "Nanami (JP)".into(), lang: "ja".into() },
                VoiceInfo { id: "zh-CN-XiaoxiaoNeural".into(), name: "Xiaoxiao (CN)".into(), lang: "zh".into() },
            ],
        }
    }

    /// Check if TTS is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Voice information.
#[derive(Debug, Clone, Serialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub lang: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TtsConfig::default();
        assert_eq!(config.provider, "edge");
        assert_eq!(config.voice, "vi-VN-HoaiMyNeural");
        assert_eq!(config.speed, 1.0);
        assert!(!config.enabled);
    }

    #[test]
    fn test_available_voices() {
        let engine = TtsEngine::new(TtsConfig::default());
        let voices = engine.available_voices();
        assert!(voices.len() >= 4);
        assert!(voices.iter().any(|v| v.lang == "vi"));
    }

    #[test]
    fn test_openai_voices() {
        let engine = TtsEngine::new(TtsConfig {
            provider: "openai".to_string(),
            ..Default::default()
        });
        let voices = engine.available_voices();
        assert!(voices.iter().any(|v| v.id == "alloy"));
    }
}

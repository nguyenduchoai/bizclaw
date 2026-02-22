//! Config Manager tool — read/write config.toml at runtime

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct ConfigManagerTool;

impl ConfigManagerTool {
    pub fn new() -> Self { Self }
}

impl Default for ConfigManagerTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for ConfigManagerTool {
    fn name(&self) -> &str { "config_manager" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "config_manager".into(),
            description: "Read or modify the BizClaw config.toml at runtime. Can get/set individual fields or read the entire config.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read", "get", "set", "list_keys"],
                        "description": "Action: read (full config), get (single key), set (update key), list_keys (show all keys)"
                    },
                    "key": {
                        "type": "string",
                        "description": "Config key in dot notation (e.g., 'default_model', 'brain.threads', 'identity.name')"
                    },
                    "value": {
                        "type": "string",
                        "description": "New value to set (for 'set' action)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let action = args["action"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'action'".into()))?;

        let config_path = bizclaw_core::config::BizClawConfig::default_path();

        match action {
            "read" => {
                // Read and display full config
                if config_path.exists() {
                    let content = tokio::fs::read_to_string(&config_path).await
                        .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Read failed: {e}")))?;

                    // Mask sensitive fields
                    let masked = mask_secrets(&content);
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Config path: {}\n\n{}", config_path.display(), masked),
                        success: true,
                    })
                } else {
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Config file not found at {}", config_path.display()),
                        success: false,
                    })
                }
            }

            "get" => {
                let key = args["key"].as_str()
                    .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'key' for get action".into()))?;

                let config = bizclaw_core::config::BizClawConfig::load()
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Load config: {e}")))?;

                let json = serde_json::to_value(&config)
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Serialize: {e}")))?;

                // Navigate dot-separated key path
                let value = get_nested_value(&json, key);

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: match value {
                        Some(v) => format!("{key} = {}", serde_json::to_string_pretty(&v).unwrap_or_default()),
                        None => format!("Key '{key}' not found in config"),
                    },
                    success: value.is_some(),
                })
            }

            "set" => {
                let key = args["key"].as_str()
                    .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'key' for set action".into()))?;
                let value = args["value"].as_str()
                    .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'value' for set action".into()))?;

                // Safety: don't allow setting certain sensitive fields via tool
                if key.contains("api_key") || key.contains("password") || key.contains("secret") {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Cannot set sensitive field '{}' via config_manager tool. Use the Dashboard UI instead.", key),
                        success: false,
                    });
                }

                let mut config = bizclaw_core::config::BizClawConfig::load()
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Load config: {e}")))?;

                let mut json = serde_json::to_value(&config)
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Serialize: {e}")))?;

                // Parse the new value
                let new_value = serde_json::from_str::<serde_json::Value>(value)
                    .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));

                // Set nested value
                set_nested_value(&mut json, key, new_value.clone());

                // Deserialize back to config
                config = serde_json::from_value(json)
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid value: {e}")))?;

                config.save()
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Save failed: {e}")))?;

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Updated: {} = {}", key, new_value),
                    success: true,
                })
            }

            "list_keys" => {
                let config = bizclaw_core::config::BizClawConfig::load()
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Load config: {e}")))?;

                let json = serde_json::to_value(&config)
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Serialize: {e}")))?;

                let keys = collect_keys(&json, "");
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Config keys ({}):\n{}", keys.len(), keys.join("\n")),
                    success: true,
                })
            }

            _ => Err(bizclaw_core::error::BizClawError::Tool(format!("Unknown action: {action}"))),
        }
    }
}

fn get_nested_value<'a>(json: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = json;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

fn set_nested_value(json: &mut serde_json::Value, key: &str, value: serde_json::Value) {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() == 1 {
        if let Some(obj) = json.as_object_mut() {
            obj.insert(parts[0].to_string(), value);
        }
        return;
    }
    // For nested paths, navigate to parent then set
    let parent_parts = &parts[..parts.len() - 1];
    let last_key = parts.last().unwrap();
    
    let mut current = json;
    for part in parent_parts {
        if !current.is_object() { return; }
        if current.get(*part).is_none() {
            current.as_object_mut().unwrap().insert(part.to_string(), serde_json::json!({}));
        }
        current = current.get_mut(*part).unwrap();
    }
    if let Some(obj) = current.as_object_mut() {
        obj.insert(last_key.to_string(), value);
    }
}

fn collect_keys(json: &serde_json::Value, prefix: &str) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(obj) = json.as_object() {
        for (k, v) in obj {
            let full_key = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{prefix}.{k}")
            };
            if v.is_object() {
                keys.extend(collect_keys(v, &full_key));
            } else {
                keys.push(full_key);
            }
        }
    }
    keys
}

fn mask_secrets(content: &str) -> String {
    let mut masked = String::new();
    for line in content.lines() {
        if line.contains("api_key") || line.contains("password") || line.contains("secret") || line.contains("token") {
            if let Some(eq_pos) = line.find('=') {
                masked.push_str(&line[..eq_pos + 1]);
                masked.push_str(" \"••••••••\"");
            } else {
                masked.push_str(line);
            }
        } else {
            masked.push_str(line);
        }
        masked.push('\n');
    }
    masked
}

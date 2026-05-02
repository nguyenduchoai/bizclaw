use crate::cdp::CdpClient;
use crate::error::{BrowserError, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub screenshot: Option<String>,
}

impl BrowserToolResult {
    pub fn ok(data: impl Into<serde_json::Value>) -> Self {
        Self {
            success: true,
            data: Some(data.into()),
            error: None,
            screenshot: None,
        }
    }
    
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            screenshot: None,
        }
    }
    
    pub fn with_screenshot(mut self, screenshot: String) -> Self {
        self.screenshot = Some(screenshot);
        self
    }
}

pub struct BrowserTools {
    client: CdpClient,
    include_screenshots: bool,
}

impl BrowserTools {
    pub fn new(client: CdpClient) -> Self {
        Self {
            client,
            include_screenshots: false,
        }
    }
    
    pub fn with_screenshots(mut self, enabled: bool) -> Self {
        self.include_screenshots = enabled;
        self
    }
    
    pub async fn navigate(&self, url: &str) -> Result<BrowserToolResult> {
        info!("Navigating to: {}", url);
        
        let frame_id = self.client.navigate(url).await
            .map_err(|e| BrowserError::NavigationFailed(e.to_string()))?;
        
        self.wait_for_load().await?;
        
        let result = BrowserToolResult::ok(serde_json::json!({
            "url": url,
            "frameId": frame_id,
            "message": format!("Navigated to {}", url)
        }));
        
        if self.include_screenshots {
            self.screenshot().await.map(|ss| result.with_screenshot(ss))
        } else {
            Ok(result)
        }
    }
    
    pub async fn click(&self, selector: &str) -> Result<BrowserToolResult> {
        debug!("Clicking: {}", selector);
        
        let node_id = self.find_element(selector).await?;
        
        let params = serde_json::json!({
            "nodeId": node_id
        });
        
        self.client.send_command("DOM.scrollIntoViewIfNeeded", Some(params)).await?;
        
        let click_params = serde_json::json!({
            "type": "mousePressed",
            "x": 0,
            "y": 0,
            "button": "left",
            "clickCount": 1
        });
        self.client.send_command("Input.dispatchMouseEvent", Some(click_params)).await?;
        
        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": 0,
            "y": 0,
            "button": "left",
            "clickCount": 1
        });
        self.client.send_command("Input.dispatchMouseEvent", Some(release_params)).await?;
        
        let result = BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "message": format!("Clicked on {}", selector)
        }));
        
        if self.include_screenshots {
            self.screenshot().await.map(|ss| result.with_screenshot(ss))
        } else {
            Ok(result)
        }
    }
    
    pub async fn click_at(&self, selector: &str, x: i32, y: i32) -> Result<BrowserToolResult> {
        debug!("Clicking at {} ({}, {})", selector, x, y);
        
        let _node_id = self.find_element(selector).await?;
        
        let params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1
        });
        self.client.send_command("Input.dispatchMouseEvent", Some(params)).await?;
        
        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1
        });
        self.client.send_command("Input.dispatchMouseEvent", Some(release_params)).await?;
        
        let result = BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "x": x,
            "y": y,
            "message": format!("Clicked at ({}, {}) on {}", x, y, selector)
        }));
        
        if self.include_screenshots {
            self.screenshot().await.map(|ss| result.with_screenshot(ss))
        } else {
            Ok(result)
        }
    }
    
    pub async fn type_text(&self, selector: &str, text: &str) -> Result<BrowserToolResult> {
        debug!("Typing in {}: {}", selector, text);
        
        let node_id = self.find_element(selector).await?;
        
        let focus_params = serde_json::json!({
            "nodeId": node_id
        });
        self.client.send_command("DOM.focus", Some(focus_params)).await?;
        
        let _clear_params = serde_json::json!({
            "nodeId": node_id
        });
        self.client.send_command("Runtime.callFunctionOn", Some(serde_json::json!({
            "functionDeclaration": "function() { this.value = ''; }",
            "objectId": null
        }))).await?;
        
        for c in text.chars() {
            let key_params = serde_json::json!({
                "type": "keyDown",
                "text": c.to_string(),
                "key": c.to_string()
            });
            self.client.send_command("Input.dispatchKeyEvent", Some(key_params)).await?;
            
            let key_up_params = serde_json::json!({
                "type": "keyUp",
                "text": c.to_string(),
                "key": c.to_string()
            });
            self.client.send_command("Input.dispatchKeyEvent", Some(key_up_params)).await?;
        }
        
        let result = BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "text": text,
            "message": format!("Typed '{}' in {}", text, selector)
        }));
        
        if self.include_screenshots {
            self.screenshot().await.map(|ss| result.with_screenshot(ss))
        } else {
            Ok(result)
        }
    }
    
    pub async fn press_key(&self, key: &str) -> Result<BrowserToolResult> {
        debug!("Pressing key: {}", key);
        
        let params = serde_json::json!({
            "type": "keyDown",
            "key": key
        });
        self.client.send_command("Input.dispatchKeyEvent", Some(params)).await?;
        
        let release_params = serde_json::json!({
            "type": "keyUp",
            "key": key
        });
        self.client.send_command("Input.dispatchKeyEvent", Some(release_params)).await?;
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "key": key,
            "message": format!("Pressed key: {}", key)
        })))
    }
    
    pub async fn get_text(&self, selector: &str) -> Result<BrowserToolResult> {
        debug!("Getting text from: {}", selector);
        
        let document = self.client.send_command("DOM.getDocument", None).await?;
        let root_node_id = document.get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|n| n.as_i64())
            .ok_or_else(|| BrowserError::CdpError("Failed to get root node".to_string()))? as u32;
        
        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector
        });
        
        let query_result = self.client.send_command("DOM.querySelector", Some(query_params)).await?;
        let node_id = query_result.get("nodeId")
            .and_then(|n| n.as_i64())
            .ok_or_else(|| BrowserError::ElementNotFound(selector.to_string()))? as u32;
        
        let text_params = serde_json::json!({
            "nodeId": node_id
        });
        
        let _text_result = self.client.send_command("DOM.getBoxModel", Some(text_params)).await?;
        
        let text = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": format!("document.querySelector('{}').textContent", selector.replace('\'', "\\'"))
        }))).await?;
        
        let text_content = text.get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "text": text_content.trim(),
            "message": format!("Got text from {}", selector)
        })))
    }
    
    pub async fn get_attribute(&self, selector: &str, attribute: &str) -> Result<BrowserToolResult> {
        debug!("Getting attribute '{}' from: {}", attribute, selector);
        
        let js = format!(
            "document.querySelector('{}').getAttribute('{}')",
            selector.replace('\'', "\\'"),
            attribute
        );
        
        let result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": js
        }))).await?;
        
        let value = result.get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "attribute": attribute,
            "value": value,
            "message": format!("Got {} from {}", attribute, selector)
        })))
    }
    
    pub async fn extract_content(&self, selector: &str) -> Result<BrowserToolResult> {
        debug!("Extracting content from: {}", selector);
        
        let js = format!(
            r#"JSON.stringify({{
                text: document.querySelector('{}')?.textContent?.trim() || '',
                html: document.querySelector('{}')?.innerHTML || '',
                href: document.querySelector('{}')?.href || '',
                src: document.querySelector('{}')?.src || '',
                links: Array.from(document.querySelectorAll('a')).map(a => ({{
                    text: a.textContent?.trim(),
                    href: a.href
                }})).filter(l => l.href),
                images: Array.from(document.querySelectorAll('img')).map(img => ({{
                    alt: img.alt,
                    src: img.src
                }}))
            }})"#,
            selector.replace('\'', "\\'"),
            selector.replace('\'', "\\'"),
            selector.replace('\'', "\\'"),
            selector.replace('\'', "\\'")
        );
        
        let result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": js
        }))).await?;
        
        let content = result.get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!({}));
        
        Ok(BrowserToolResult::ok(content))
    }
    
    pub async fn extract_all(&self, tag: &str) -> Result<BrowserToolResult> {
        debug!("Extracting all {} elements", tag);
        
        let js = format!(
            r#"JSON.stringify(Array.from(document.querySelectorAll('{}')).map(el => ({{
                text: el.textContent?.trim(),
                href: el.href || el.getAttribute('href'),
                src: el.src || el.getAttribute('src')
            }})))"#,
            tag
        );
        
        let result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": js
        }))).await?;
        
        let elements = result.get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::Value::Array(vec![]));
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "tag": tag,
            "count": if let serde_json::Value::Array(arr) = &elements { arr.len() } else { 0 },
            "elements": elements
        })))
    }
    
    pub async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<BrowserToolResult> {
        debug!("Waiting for selector: {} (timeout: {}ms)", selector, timeout_ms);
        
        let start = std::time::Instant::now();
        let interval = std::time::Duration::from_millis(500);
        
        while start.elapsed().as_millis() < timeout_ms as u128 {
            let js = format!("document.querySelector('{}') !== null", selector.replace('\'', "\\'"));
            
            let result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
                "expression": js
            }))).await;
            
            if let Ok(resp) = result {
                if resp.get("result")
                    .and_then(|r| r.get("value"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    return Ok(BrowserToolResult::ok(serde_json::json!({
                        "selector": selector,
                        "found": true,
                        "message": format!("Found selector {}", selector)
                    })));
                }
            }
            
            tokio::time::sleep(interval).await;
        }
        
        Err(BrowserError::Timeout(format!("Selector {} not found within {}ms", selector, timeout_ms)))
    }
    
    pub async fn wait_for_navigation(&self, timeout_ms: u64) -> Result<BrowserToolResult> {
        debug!("Waiting for navigation (timeout: {}ms)", timeout_ms);
        
        let params = serde_json::json!({
            "waitForNavigation": true,
            "timeout": timeout_ms
        });
        
        self.client.send_command("Page.setLifecycleEventsEnabled", Some(params)).await?;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(timeout_ms)).await;
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "navigated": true,
            "message": "Navigation completed"
        })))
    }
    
    pub async fn get_page_info(&self) -> Result<BrowserToolResult> {
        debug!("Getting page info");
        
        let url_result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": "JSON.stringify({ url: window.location.href, title: document.title })"
        }))).await?;
        
        let info = url_result.get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(serde_json::json!({}));
        
        Ok(BrowserToolResult::ok(info))
    }
    
    pub async fn scroll(&self, x: i32, y: i32) -> Result<BrowserToolResult> {
        debug!("Scrolling to ({}, {})", x, y);
        
        let params = serde_json::json!({
            "type": "mouseWheel",
            "x": x,
            "y": y,
            "deltaX": 0,
            "deltaY": 0
        });
        
        self.client.send_command("Input.dispatchMouseEvent", Some(params)).await?;
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "x": x,
            "y": y,
            "message": format!("Scrolled to ({}, {})", x, y)
        })))
    }
    
    pub async fn scroll_element(&self, selector: &str, direction: &str) -> Result<BrowserToolResult> {
        debug!("Scrolling {} in {}", selector, direction);
        
        let js = format!(
            "document.querySelector('{}').scrollBy(0, {})",
            selector.replace('\'', "\\'"),
            match direction {
                "down" => "300",
                "up" => "-300",
                _ => "300"
            }
        );
        
        self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": js
        }))).await?;
        
        if self.include_screenshots {
            self.screenshot().await.map(|ss| {
                BrowserToolResult::ok(serde_json::json!({
                    "selector": selector,
                    "direction": direction
                })).with_screenshot(ss)
            })
        } else {
            Ok(BrowserToolResult::ok(serde_json::json!({
                "selector": selector,
                "direction": direction
            })))
        }
    }
    
    pub async fn hover(&self, selector: &str) -> Result<BrowserToolResult> {
        debug!("Hovering over: {}", selector);
        
        let _node_id = self.find_element(selector).await?;
        
        let params = serde_json::json!({
            "type": "mouseMoved",
            "x": 0,
            "y": 0
        });
        
        self.client.send_command("Input.dispatchMouseEvent", Some(params)).await?;
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "message": format!("Hovered over {}", selector)
        })))
    }
    
    pub async fn select_option(&self, selector: &str, value: &str) -> Result<BrowserToolResult> {
        debug!("Selecting option '{}' in {}", value, selector);
        
        let js = format!(
            r#"var select = document.querySelector('{}');
            if (select) {{
                select.value = '{}';
                select.dispatchEvent(new Event('change', {{ bubbles: true }}));
                'selected';
            }} else {{ 'not found'; }}"#,
            selector.replace('\'', "\\'"),
            value
        );
        
        let result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": js
        }))).await?;
        
        let success = result.get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .map(|s| s == "selected")
            .unwrap_or(false);
        
        if success {
            Ok(BrowserToolResult::ok(serde_json::json!({
                "selector": selector,
                "value": value,
                "message": format!("Selected option '{}'", value)
            })))
        } else {
            Err(BrowserError::ElementNotFound(format!("Could not select '{}' in {}", value, selector)))
        }
    }
    
    pub async fn check(&self, selector: &str, checked: bool) -> Result<BrowserToolResult> {
        debug!("Setting checkbox {} to {}", selector, checked);
        
        let js = format!(
            r#"var el = document.querySelector('{}');
            if (el) {{
                el.checked = {};
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                'done';
            }} else {{ 'not found'; }}"#,
            selector.replace('\'', "\\'"),
            checked
        );
        
        let _result = self.client.send_command("Runtime.evaluate", Some(serde_json::json!({
            "expression": js
        }))).await?;
        
        Ok(BrowserToolResult::ok(serde_json::json!({
            "selector": selector,
            "checked": checked
        })))
    }
    
    async fn find_element(&self, selector: &str) -> Result<u32> {
        let document = self.client.send_command("DOM.getDocument", None).await?;
        let root_node_id = document.get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|n| n.as_i64())
            .ok_or_else(|| BrowserError::CdpError("Failed to get root node".to_string()))? as u32;
        
        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector
        });
        
        let query_result = self.client.send_command("DOM.querySelector", Some(query_params)).await?;
        let node_id = query_result.get("nodeId")
            .and_then(|n| n.as_i64())
            .ok_or_else(|| BrowserError::ElementNotFound(selector.to_string()))? as u32;
        
        Ok(node_id)
    }
    
    async fn wait_for_load(&self) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        Ok(())
    }
    
    pub async fn screenshot(&self) -> Result<String> {
        let params = serde_json::json!({
            "format": "png",
            "fromSurface": true
        });
        
        let result = self.client.send_command("Page.captureScreenshot", Some(params)).await?;
        
        let data = result.get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| BrowserError::ScreenshotFailed("No data returned".to_string()))?;
        
        Ok(data.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_browser_tool_result() {
        let result = BrowserToolResult::ok(serde_json::json!({"test": "value"}));
        assert!(result.success);
        assert!(result.data.is_some());
        
        let error = BrowserToolResult::err("test error");
        assert!(!error.success);
        assert!(error.error.is_some());
    }
}

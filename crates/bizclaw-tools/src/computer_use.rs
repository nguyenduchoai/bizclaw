//! # Computer Use Tool - Điều khiển máy tính
//!
//! Native computer control cho BizClaw - không cần browser.
//!
//! ## Actions
//! - `screenshot`: Chụp màn hình, resize về 1024px JPEG q30
//! - `mouse_move`: Di chuyển chuột đến vị trí (x, y)
//! - `mouse_click`: Click chuột (left, right, double)
//! - `type_text`: Gõ text vào vị trí hiện tại
//! - `key_press`: Nhấn phím đặc biệt
//! - `drag`: Kéo thả
//!
//! ## Platform Support
//! - macOS: screencapture + cliclick
//! - Windows: PowerShell + user32.dll
//! - Linux: scrot/import + xdotool

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::process::Command;

pub struct ComputerUseTool {
    config: ComputerUseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseConfig {
    pub screenshot_quality: u8,
    pub screenshot_max_width: u32,
    pub default_delay_ms: u64,
}

impl Default for ComputerUseConfig {
    fn default() -> Self {
        Self {
            screenshot_quality: 30,
            screenshot_max_width: 1024,
            default_delay_ms: 100,
        }
    }
}

impl ComputerUseTool {
    pub fn new() -> Self {
        Self {
            config: ComputerUseConfig::default(),
        }
    }

    pub fn with_config(config: ComputerUseConfig) -> Self {
        Self { config }
    }

    async fn screenshot(&self) -> std::result::Result<String, String> {
        let temp_path = std::env::temp_dir().join("bizclaw_screenshot.png");
        let temp_path_str = temp_path.to_string_lossy().to_string();

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("screencapture")
                .args(["-x", &temp_path_str])
                .output()
                .map_err(|e| format!("Failed to run screencapture: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "screencapture failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $screen = [System.Windows.Forms.Screen]::PrimaryScreen; $bounds = $screen.Bounds; $bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height); $graphics = [System.Drawing.Graphics]::FromImage($bitmap); $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size); $bitmap.Save('{}'); $graphics.Dispose(); $bitmap.Dispose()"#,
                temp_path_str.replace('\\', "\\\\")
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "PowerShell screenshot failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        #[cfg(target_os = "linux")]
        {
            let result = Command::new("scrot").arg(&temp_path_str).output();

            if result.is_err() || !result.as_ref().unwrap().status.success() {
                let output = Command::new("import")
                    .args(["-window", "root", &temp_path_str])
                    .output()
                    .map_err(|e| format!("Failed to run import: {e}"))?;

                if !output.status.success() {
                    return Err("Both scrot and import failed".to_string());
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            return Err("Unsupported platform for screenshot".to_string());
        }

        if !temp_path.exists() {
            return Err("Screenshot file not created".to_string());
        }

        let resized_path = std::env::temp_dir().join("bizclaw_screenshot_resized.jpg");
        self.resize_and_convert(&temp_path, &resized_path)?;

        let base64 = std::fs::read_to_string(&resized_path)
            .map_err(|e| format!("Failed to read screenshot: {e}"))?;

        let _ = std::fs::remove_file(&temp_path);
        let _ = std::fs::remove_file(&resized_path);

        Ok(base64)
    }

    fn resize_and_convert(
        &self,
        input: &std::path::Path,
        output: &std::path::Path,
    ) -> std::result::Result<(), String> {
        let img = image::open(input).map_err(|e| format!("Failed to open image: {e}"))?;

        let (width, height) = img.dimensions();
        let max_width = self.config.screenshot_max_width;

        let img = if width > max_width {
            let ratio = max_width as f32 / width as f32;
            let new_height = (height as f32 * ratio) as u32;
            img.resize(max_width, new_height, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        img.save_with_format(output, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Failed to save JPEG: {e}"))?;

        Ok(())
    }

    async fn mouse_move(&self, x: i32, y: i32) -> std::result::Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("cliclick")
                .args(["m:#{x},{y}"
                    .replace("{x}", &x.to_string())
                    .replace("{y}", &y.to_string())])
                .output()
                .map_err(|e| format!("Failed to run cliclick: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "cliclick failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})",
                x, y
            );
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "PowerShell mouse_move failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("xdotool")
                .args(["mousemove", "--sync", &x.to_string(), &y.to_string()])
                .output()
                .map_err(|e| format!("Failed to run xdotool: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "xdotool failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            return Err("Unsupported platform".to_string());
        }

        Ok(())
    }

    async fn mouse_click(
        &self,
        x: i32,
        y: i32,
        button: &str,
        clicks: u32,
    ) -> std::result::Result<(), String> {
        self.mouse_move(x, y).await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let click_args = match (button, clicks) {
            ("right", 1) => "rc".to_string(),
            ("right", 2) => "rc rc".to_string(),
            ("middle", 1) => "mc".to_string(),
            ("middle", 2) => "mc mc".to_string(),
            (_, 1) => "c".to_string(),
            (_, 2) => "dc".to_string(),
            (_, 3) => "tc".to_string(),
            _ => "c".to_string(),
        };

        #[cfg(target_os = "macos")]
        {
            let pos_part = format!("{}:{}", x, y);
            for click in click_args.split_whitespace() {
                let output = Command::new("cliclick")
                    .args([click, &format!("c:{pos_part}")])
                    .output()
                    .map_err(|e| format!("Failed to run cliclick: {e}"))?;

                if !output.status.success() {
                    return Err(format!("cliclick {} failed", click));
                }

                if click != click_args.split_whitespace().last().unwrap_or(click) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let (down, up) = match button {
                "right" => ("0x0008", "0x0010"),
                "middle" => ("0x0020", "0x0040"),
                _ => ("0x0002", "0x0004"),
            };

            let script = format!(
                r#"Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinClick {{
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint f, uint dx, uint dy, uint d, int e);
    public static void Click(int x, int y, uint down, uint up, int n) {{
        SetCursorPos(x, y);
        for (int i = 0; i < n; i++) {{
            mouse_event(down, 0, 0, 0, 0);
            mouse_event(up, 0, 0, 0, 0);
            if (i < n - 1) System.Threading.Thread.Sleep(50);
        }}
    }}
}}
"@
[WinClick]::Click({}, {}, {}, {}, {})"#,
                x, y, down, up, clicks
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                return Err(format!(
                    "PowerShell mouse_click failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        #[cfg(target_os = "linux")]
        {
            let button_num = match button {
                "right" => "3",
                "middle" => "2",
                _ => "1",
            };

            let output = Command::new("xdotool")
                .args([
                    "mousemove",
                    "--sync",
                    &x.to_string(),
                    &y.to_string(),
                    "click",
                    button_num,
                ])
                .output()
                .map_err(|e| format!("Failed to run xdotool: {e}"))?;

            if !output.status.success() {
                return Err(format!("xdotool click failed"));
            }
        }

        Ok(())
    }

    async fn type_text(&self, text: &str) -> std::result::Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
            let output = Command::new("cliclick")
                .args(["t:"])
                .output()
                .map_err(|e| format!("Failed to run cliclick: {e}"))?;

            if !output.status.success() {
                return Err("Failed to type text".to_string());
            }

            Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        "tell application \"System Events\" to keystroke \"{}\"",
                        escaped
                    ),
                ])
                .output()
                .map_err(|e| format!("Failed to run osascript: {e}"))?;
        }

        #[cfg(target_os = "windows")]
        {
            let escaped = text.replace('"', "`\"");
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait(\"{}\")",
                escaped
            );
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                return Err(format!("SendKeys failed"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("xdotool")
                .args(["type", "--", text])
                .output()
                .map_err(|e| format!("Failed to run xdotool: {e}"))?;

            if !output.status.success() {
                return Err(format!("xdotool type failed"));
            }
        }

        Ok(())
    }

    async fn key_press(&self, key: &str) -> std::result::Result<(), String> {
        let key_code = match key.to_lowercase().as_str() {
            "enter" | "return" => "Return",
            "escape" | "esc" => "Escape",
            "tab" => "Tab",
            "backspace" => "BackSpace",
            "delete" => "Delete",
            "up" | "uparrow" => "Up",
            "down" | "downarrow" => "Down",
            "left" | "leftarrow" => "Left",
            "right" | "rightarrow" => "Right",
            "home" => "Home",
            "end" => "End",
            "pageup" => "Page_Up",
            "pagedown" => "Page_Down",
            "space" => "Space",
            "ctrl" | "control" => "Control",
            "alt" => "Alt",
            "shift" => "Shift",
            "cmd" | "command" | "super" => "Meta",
            _ => key,
        };

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("cliclick")
                .args(["kp", key_code])
                .output()
                .map_err(|e| format!("Failed to run cliclick: {e}"))?;

            if !output.status.success() {
                return Err(format!("cliclick keypress failed"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let vk_code = match key.to_lowercase().as_str() {
                "enter" | "return" => "{ENTER}",
                "escape" | "esc" => "{ESC}",
                "tab" => "{TAB}",
                "backspace" => "{BACKSPACE}",
                "delete" => "{DELETE}",
                "up" | "uparrow" => "{UP}",
                "down" | "downarrow" => "{DOWN}",
                "left" | "leftarrow" => "{LEFT}",
                "right" | "rightarrow" => "{RIGHT}",
                "home" => "{HOME}",
                "end" => "{END}",
                "pageup" => "{PGUP}",
                "pagedown" => "{PGDN}",
                "space" => " ",
                "ctrl" | "control" => "^",
                "alt" => "%",
                "shift" => "+",
                _ => key,
            };

            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait(\"{}\")",
                vk_code
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                return Err(format!("SendKeys keypress failed"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("xdotool")
                .args(["key", key_code])
                .output()
                .map_err(|e| format!("Failed to run xdotool: {e}"))?;

            if !output.status.success() {
                return Err(format!("xdotool keypress failed"));
            }
        }

        Ok(())
    }

    async fn drag(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> std::result::Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("cliclick")
                .args([
                    "dd:#{x1},{y1}"
                        .replace("{x1}", &x1.to_string())
                        .replace("{y1}", &y1.to_string())
                        .as_str(),
                    "du:#{x2},{y2}"
                        .replace("{x2}", &x2.to_string())
                        .replace("{y2}", &y2.to_string())
                        .as_str(),
                ])
                .output()
                .map_err(|e| format!("Failed to run cliclick: {e}"))?;

            if !output.status.success() {
                return Err(format!("cliclick drag failed"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Drag {{
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint f, uint dx, uint dy, uint d, int e);
    public static void DragTo(int x1, int y1, int x2, int y2) {{
        SetCursorPos(x1, y1);
        mouse_event(0x0002, 0, 0, 0, 0);
        System.Threading.Thread.Sleep(100);
        for (int i = 0; i <= 10; i++) {{
            SetCursorPos(x1 + (x2-x1)*i/10, y1 + (y2-y1)*i/10);
            System.Threading.Thread.Sleep(10);
        }}
        mouse_event(0x0004, 0, 0, 0, 0);
    }}
}}
"@
[Drag]::DragTo({}, {}, {}, {})"#,
                x1, y1, x2, y2
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &script])
                .output()
                .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

            if !output.status.success() {
                return Err(format!("PowerShell drag failed"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("xdotool")
                .args([
                    "mousemove",
                    "--sync",
                    &x1.to_string(),
                    &y1.to_string(),
                    "mousedown",
                    "1",
                ])
                .output()
                .map_err(|e| format!("Failed to run xdotool: {e}"))?;

            if !output.status.success() {
                return Err(format!("xdotool drag start failed"));
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            for i in 0..=10 {
                let cx = x1 + (x2 - x1) * i / 10;
                let cy = y1 + (y2 - y1) * i / 10;
                let _ = Command::new("xdotool")
                    .args(["mousemove", &cx.to_string(), &cy.to_string()])
                    .output();
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }

            let output = Command::new("xdotool")
                .args(["mouseup", "1"])
                .output()
                .map_err(|e| format!("Failed to run xdotool: {e}"))?;

            if !output.status.success() {
                return Err(format!("xdotool drag end failed"));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ComputerUseArgs {
    action: String,
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    button: Option<String>,
    #[serde(default)]
    clicks: Option<u32>,
    #[serde(default)]
    x2: Option<i32>,
    #[serde(default)]
    y2: Option<i32>,
}

#[async_trait]
impl Tool for ComputerUseTool {
    fn name(&self) -> &str {
        "computer_use"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "computer_use".to_string(),
            description: "Điều khiển máy tính: chụp màn hình, di chuyển chuột, click, gõ text, nhấn phím. Sử dụng screenshot trước để xem màn hình, sau đó dùng mouse/keyboard để tương tác.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["screenshot", "mouse_move", "mouse_click", "double_click", "right_click", "type_text", "key_press", "drag"],
                        "description": "Hành động cần thực hiện"
                    },
                    "x": {"type": "integer", "description": "Tọa độ X"},
                    "y": {"type": "integer", "description": "Tọa độ Y"},
                    "text": {"type": "string", "description": "Text cần gõ"},
                    "key": {"type": "string", "description": "Phím cần nhấn (enter, escape, tab, ...)"}
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let parsed: ComputerUseArgs = serde_json::from_str(args)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        let result = match parsed.action.as_str() {
            "screenshot" => {
                let base64 = self.screenshot().await.map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "image": base64,
                    "message": "Screenshot captured. Analyze the image to determine next actions."
                })
                .to_string()
            }

            "mouse_move" => {
                let (x, y) = parsed.x.zip(parsed.y).ok_or_else(|| {
                    BizClawError::Tool("x and y coordinates required".to_string())
                })?;

                self.mouse_move(x, y)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "x": x,
                    "y": y,
                    "message": format!("Mouse moved to ({}, {})", x, y)
                })
                .to_string()
            }

            "mouse_click" | "click" => {
                let (x, y) = parsed.x.zip(parsed.y).ok_or_else(|| {
                    BizClawError::Tool("x and y coordinates required".to_string())
                })?;
                let button = parsed.button.unwrap_or_else(|| "left".to_string());
                let clicks = parsed.clicks.unwrap_or(1);

                self.mouse_click(x, y, &button, clicks)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "x": x,
                    "y": y,
                    "button": button,
                    "clicks": clicks,
                    "message": format!("{} click at ({}, {})", button, x, y)
                })
                .to_string()
            }

            "double_click" => {
                let (x, y) = parsed.x.zip(parsed.y).ok_or_else(|| {
                    BizClawError::Tool("x and y coordinates required".to_string())
                })?;

                self.mouse_click(x, y, "left", 2)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "x": x,
                    "y": y,
                    "message": format!("Double click at ({}, {})", x, y)
                })
                .to_string()
            }

            "right_click" => {
                let (x, y) = parsed.x.zip(parsed.y).ok_or_else(|| {
                    BizClawError::Tool("x and y coordinates required".to_string())
                })?;

                self.mouse_click(x, y, "right", 1)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "x": x,
                    "y": y,
                    "message": format!("Right click at ({}, {})", x, y)
                })
                .to_string()
            }

            "type_text" | "type" => {
                let text = parsed
                    .text
                    .ok_or_else(|| BizClawError::Tool("text required".to_string()))?;

                self.type_text(&text)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "text": text,
                    "message": format!("Typed: {}", text)
                })
                .to_string()
            }

            "key_press" | "key" => {
                let key = parsed
                    .key
                    .ok_or_else(|| BizClawError::Tool("key required".to_string()))?;

                self.key_press(&key)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "key": key,
                    "message": format!("Pressed: {}", key)
                })
                .to_string()
            }

            "drag" => {
                let (x1, y1) = parsed.x.zip(parsed.y).ok_or_else(|| {
                    BizClawError::Tool(
                        "x and y coordinates required for start position".to_string(),
                    )
                })?;
                let (x2, y2) = parsed.x2.zip(parsed.y2).ok_or_else(|| {
                    BizClawError::Tool(
                        "x2 and y2 coordinates required for end position".to_string(),
                    )
                })?;

                self.drag(x1, y1, x2, y2)
                    .await
                    .map_err(|e| BizClawError::Tool(e))?;
                serde_json::json!({
                    "success": true,
                    "from": {"x": x1, "y": y1},
                    "to": {"x": x2, "y": y2},
                    "message": format!("Dragged from ({}, {}) to ({}, {})", x1, y1, x2, y2)
                })
                .to_string()
            }

            _ => {
                return Err(BizClawError::Tool(format!(
                    "Unknown action: {}",
                    parsed.action
                )));
            }
        };

        Ok(ToolResult {
            tool_call_id: "computer_use".to_string(),
            output: result,
            success: true,
        })
    }
}

pub fn new() -> Box<dyn Tool> {
    Box::new(ComputerUseTool::new())
}

//! Tool Loop Detection — detects and prevents infinite tool call loops.
//!
//! Monitors tool call patterns and blocks repetitive calls to save tokens.
//! Inspired by GoClaw's tool loop detection.

use std::collections::VecDeque;

/// Tool loop detector — tracks recent tool calls and detects repetitive patterns.
#[derive(Debug, Clone)]
pub struct LoopDetector {
    /// Recent tool call history: (tool_name, args_hash).
    history: VecDeque<(String, u64)>,
    /// Max history size to track.
    max_history: usize,
    /// Number of repeated calls before triggering detection.
    max_repetitions: usize,
    /// Total loops detected.
    loops_detected: u64,
    /// Total tokens saved (estimated).
    tokens_saved: u64,
}

impl LoopDetector {
    /// Create a new loop detector.
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(20),
            max_history: 20,
            max_repetitions: 3,
            loops_detected: 0,
            tokens_saved: 0,
        }
    }

    /// Create with custom thresholds.
    pub fn with_config(max_history: usize, max_repetitions: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_history),
            max_history,
            max_repetitions,
            loops_detected: 0,
            tokens_saved: 0,
        }
    }

    /// Record a tool call and check if it's a loop.
    /// Returns `true` if a loop is detected (caller should block the call).
    pub fn check(&mut self, tool_name: &str, arguments: &str) -> bool {
        let args_hash = Self::hash_args(arguments);
        let entry = (tool_name.to_string(), args_hash);

        // Count how many times this exact call appears in recent history
        let count = self
            .history
            .iter()
            .filter(|(name, hash)| name == tool_name && *hash == args_hash)
            .count();

        // Add to history
        self.history.push_back(entry);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        if count >= self.max_repetitions {
            self.loops_detected += 1;
            self.tokens_saved += 500; // Estimated tokens saved per blocked call
            tracing::warn!(
                "🔁 Loop detected: '{}' called {} times with same args (blocked)",
                tool_name,
                count + 1
            );
            return true;
        }

        // Check for alternating pattern: A → B → A → B → A → B
        if self.history.len() >= 6 {
            let len = self.history.len();
            let a = &self.history[len - 1];
            let b = &self.history[len - 2];
            if self.history[len - 3] == *a
                && self.history[len - 4] == *b
                && self.history[len - 5] == *a
                && self.history[len - 6] == *b
            {
                self.loops_detected += 1;
                self.tokens_saved += 500;
                tracing::warn!(
                    "🔁 Alternating loop detected: '{}' ↔ '{}' (blocked)",
                    a.0,
                    b.0
                );
                return true;
            }
        }

        false
    }

    /// Clear history (e.g., on new conversation).
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Get statistics.
    pub fn stats(&self) -> LoopDetectorStats {
        LoopDetectorStats {
            history_size: self.history.len(),
            loops_detected: self.loops_detected,
            tokens_saved: self.tokens_saved,
        }
    }

    /// Simple hash for arguments (FNV-1a inspired).
    fn hash_args(args: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in args.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from the loop detector.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LoopDetectorStats {
    pub history_size: usize,
    pub loops_detected: u64,
    pub tokens_saved: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_loop_detected() {
        let mut detector = LoopDetector::new();
        assert!(!detector.check("web_search", r#"{"query": "rust"}"#));
        assert!(!detector.check("file", r#"{"path": "/tmp/a"}"#));
        assert!(!detector.check("shell", r#"{"command": "ls"}"#));
    }

    #[test]
    fn test_same_call_loop() {
        let mut detector = LoopDetector::new();
        let args = r#"{"query": "rust programming"}"#;

        assert!(!detector.check("web_search", args)); // 1st
        assert!(!detector.check("web_search", args)); // 2nd
        assert!(!detector.check("web_search", args)); // 3rd
        assert!(detector.check("web_search", args)); // 4th → LOOP!

        assert_eq!(detector.stats().loops_detected, 1);
        assert!(detector.stats().tokens_saved > 0);
    }

    #[test]
    fn test_different_args_no_loop() {
        let mut detector = LoopDetector::new();
        assert!(!detector.check("web_search", r#"{"query": "rust"}"#));
        assert!(!detector.check("web_search", r#"{"query": "python"}"#));
        assert!(!detector.check("web_search", r#"{"query": "go"}"#));
        assert!(!detector.check("web_search", r#"{"query": "java"}"#));
    }

    #[test]
    fn test_alternating_loop() {
        let mut detector = LoopDetector::new();
        let args_a = r#"{"path": "/tmp/a"}"#;
        let args_b = r#"{"path": "/tmp/b"}"#;

        assert!(!detector.check("file", args_a));
        assert!(!detector.check("file", args_b));
        assert!(!detector.check("file", args_a));
        assert!(!detector.check("file", args_b));
        assert!(!detector.check("file", args_a));
        assert!(detector.check("file", args_b)); // Alternating pattern detected!
    }

    #[test]
    fn test_clear_resets() {
        let mut detector = LoopDetector::new();
        let args = r#"{"x": 1}"#;
        detector.check("a", args);
        detector.check("a", args);
        detector.check("a", args);
        detector.clear();
        // After clear, should not detect loop
        assert!(!detector.check("a", args));
    }

    #[test]
    fn test_custom_config() {
        let mut detector = LoopDetector::with_config(10, 2); // Stricter: 2 reps
        let args = r#"{"x": 1}"#;
        assert!(!detector.check("tool", args));
        assert!(!detector.check("tool", args));
        assert!(detector.check("tool", args)); // 3rd call with max_repetitions=2
    }
}

#!/bin/bash
# ════════════════════════════════════════════════════════════════
# Setup llama.cpp for BizClaw Android NDK build
#
# Clones llama.cpp as a submodule for on-device LLM inference.
# This is the industry-standard C/C++ LLM engine used by:
#   - SmolChat-Android (Google Play)
#   - llama.cpp official Android example
#
# Usage: ./setup_llama_cpp.sh
# ════════════════════════════════════════════════════════════════

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LLAMA_DIR="$SCRIPT_DIR/llama.cpp"

echo "🧠 Setting up llama.cpp for BizClaw Android..."

# Clone or update llama.cpp
if [ -d "$LLAMA_DIR" ]; then
    echo "📦 Updating existing llama.cpp..."
    cd "$LLAMA_DIR"
    git pull origin master 2>/dev/null || true
    cd "$SCRIPT_DIR"
else
    echo "📥 Cloning llama.cpp (this may take a moment)..."
    cd "$SCRIPT_DIR"
    git submodule add --depth 1 https://github.com/ggerganov/llama.cpp.git llama.cpp 2>/dev/null \
        || git clone --depth 1 https://github.com/ggerganov/llama.cpp.git llama.cpp
fi

# Verify critical files
REQUIRED=("llama.cpp/CMakeLists.txt" "llama.cpp/include/llama.h" "llama.cpp/common/common.h")
MISSING=0
for f in "${REQUIRED[@]}"; do
    if [ ! -f "$f" ]; then
        echo "⚠️  Missing: $f"
        MISSING=$((MISSING + 1))
    fi
done

if [ $MISSING -eq 0 ]; then
    echo "✅ llama.cpp ready!"
    echo ""
    echo "📊 Source stats:"
    echo "  llama.cpp version: $(cd llama.cpp && git describe --tags 2>/dev/null || echo 'latest')"
    echo "  Total C/C++ files: $(find llama.cpp -name '*.cpp' -o -name '*.c' -o -name '*.h' | wc -l | tr -d ' ')"
    echo ""
    echo "🔨 Build: ./gradlew assembleDebug"
else
    echo "❌ $MISSING files missing"
    exit 1
fi

echo ""
echo "════════════════════════════════════════════════════════"
echo "  llama.cpp setup complete!"
echo ""
echo "  Supported GGUF models:"
echo "    • Qwen3 4B Q4_K_M  — 2.7 GB — Best balance"
echo "    • Qwen3 8B Q4_K_M  — 5.1 GB — Powerful (NPU phones)"
echo "    • TinyLlama 1.1B   — 638 MB — Any phone"
echo "    • DeepSeek R1 1.5B — 1.1 GB — Reasoning"
echo "    • Phi-4 Mini 3.8B  — 2.4 GB — Microsoft reasoning"
echo ""
echo "  CPU optimizations (auto-detected at runtime):"
echo "    • ARMv8.2: fp16, dotprod"
echo "    • ARMv8.4: fp16, dotprod, SVE, i8mm"
echo ""
echo "  Build: cd android && ./gradlew assembleDebug"
echo "════════════════════════════════════════════════════════"

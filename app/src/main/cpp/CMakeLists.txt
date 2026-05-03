cmake_minimum_required(VERSION 3.22.1)
project("bizclaw_llm")

# ════════════════════════════════════════════════════════════════
# BizClaw LLM — on-device inference via llama.cpp
#
# Architecture: Same as SmolChat-Android
#   llama.cpp (submodule) → LLMInference.cpp → bizclaw_llm_jni.cpp → Kotlin
#
# Build variants by CPU feature for maximum performance:
#   - bizclaw_llm           (universal fallback)
#   - bizclaw_llm_v8        (ARMv8-A baseline)
#   - bizclaw_llm_v8_2_*    (ARMv8.2-A + fp16/dotprod)
#   - bizclaw_llm_v8_4_*    (ARMv8.4-A + fp16/dotprod/i8mm/sve)
#
# Reference: github.com/shubham0204/SmolChat-Android
# ════════════════════════════════════════════════════════════════

# llama.cpp as submodule at android/llama.cpp/
add_subdirectory(../../../../llama.cpp llama.cpp)
set(LLAMA_DIR_RELATIVE "../../../../llama.cpp")
get_filename_component(LLAMA_DIR ${LLAMA_DIR_RELATIVE} ABSOLUTE)

set(GGML_DIR ${LLAMA_DIR}/ggml)
set(COMMON_DIR ${LLAMA_DIR}/common)
set(VENDOR_DIR ${LLAMA_DIR}/vendor)

# Hide symbols + LTO
target_compile_options(llama PUBLIC -fvisibility=hidden -fvisibility-inlines-hidden)
target_compile_options(llama PUBLIC -ffunction-sections -fdata-sections)
target_link_options(llama PRIVATE -Wl,--gc-sections -flto -Wl,--exclude-libs,ALL)

# ── Build function: core library ──
function(build_library target_name)
    add_library(${target_name} SHARED
        LLMInference.cpp
        bizclaw_llm_jni.cpp
    )
    target_include_directories(${target_name} PUBLIC
        ${COMMON_DIR}
        ${GGML_DIR}/include
        ${GGML_DIR}/src
        ${GGML_DIR}/src/ggml-cpu
        ${LLAMA_DIR}/include
        ${VENDOR_DIR}
    )
    target_compile_options(${target_name} PUBLIC
        -fvisibility=hidden -fvisibility-inlines-hidden
        -ffunction-sections -fdata-sections
    )
    target_link_libraries(${target_name} android log llama common)
    target_link_options(${target_name} PRIVATE
        -Wl,--gc-sections -flto -Wl,--exclude-libs,ALL
    )
endfunction()

# ── ARM64 variant builder ──
function(build_library_arm64 target_name cpu_flags)
    build_library(${target_name})
    target_compile_options(${target_name} PUBLIC
        -DGGML_USE_CPU -DGGML_USE_CPU_AARCH64 ${cpu_flags} -O3
    )
endfunction()

# ── Universal variant (x86_64 emulator, fallback) ──
function(build_library_universal target_name)
    build_library(${target_name})
    target_compile_options(${target_name} PUBLIC -DGGML_USE_CPU -O3)
endfunction()

# ────────────────────────────────────────────────────────────
# Build all variants
# ────────────────────────────────────────────────────────────

# Universal fallback (always built)
build_library_universal("bizclaw_llm")

if (${ANDROID_ABI} STREQUAL "arm64-v8a")
    # ARMv8-A baseline
    build_library_arm64("bizclaw_llm_v8" "-march=armv8-a")
    # ARMv8.2-A variants
    build_library_arm64("bizclaw_llm_v8_2_fp16" "-march=armv8.2-a+fp16")
    build_library_arm64("bizclaw_llm_v8_2_fp16_dotprod" "-march=armv8.2-a+fp16+dotprod")
    # ARMv8.4-A variants (high-end phones: Snapdragon 8 Gen 1+, Dimensity 9000+)
    build_library_arm64("bizclaw_llm_v8_4_fp16_dotprod" "-march=armv8.4-a+fp16+dotprod")
    build_library_arm64("bizclaw_llm_v8_4_fp16_dotprod_sve" "-march=armv8.4-a+fp16+dotprod+sve")
    build_library_arm64("bizclaw_llm_v8_4_fp16_dotprod_i8mm" "-march=armv8.4-a+fp16+dotprod+i8mm")
    build_library_arm64("bizclaw_llm_v8_4_fp16_dotprod_i8mm_sve" "-march=armv8.4-a+fp16+dotprod+i8mm+sve")
endif()

# ── GGUF Reader (separate small library for metadata only) ──
set(TARGET_NAME_GGUF_READER bizclaw_ggufreader)
add_library(${TARGET_NAME_GGUF_READER} SHARED GGUFReader.cpp)
target_include_directories(${TARGET_NAME_GGUF_READER} PUBLIC ${GGML_DIR}/include)
target_compile_options(${TARGET_NAME_GGUF_READER} PUBLIC
    -fvisibility=hidden -fvisibility-inlines-hidden -ffunction-sections -fdata-sections
)
target_link_libraries(${TARGET_NAME_GGUF_READER} ggml)
target_link_options(${TARGET_NAME_GGUF_READER} PRIVATE
    -Wl,--gc-sections -flto -Wl,--exclude-libs,ALL
)

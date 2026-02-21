# PicoLM Analysis Report

> **Source**: https://github.com/RightNow-AI/picolm
> **Language**: C11 (~2,500 lines)
> **License**: MIT
> **Tagline**: "Run a 1-billion parameter LLM on a $10 board with 256MB RAM"

## 1. Overview

PicoLM là một **minimal LLM inference engine** viết từ đầu bằng C11:
- **Pure C, zero dependencies, one binary**
- **No Python, No cloud**
- Chạy TinyLlama 1.1B (và các LLaMA-architecture models GGUF format)
- Target: Raspberry Pi Zero 2W ($15), Sipeed LicheeRV ($12), RPi 3/4/5

## 2. Key Metrics

| Metric | Value |
|--------|-------|
| Runtime RAM | ~45 MB (including FP16 KV cache) |
| Binary size | ~80 KB |
| Model file | 638 MB (stays on disk via mmap) |
| Source code | ~2,500 lines C11 |
| Dependencies | Zero (pure C) |
| Physical memory used | ~30 MB at any time |

## 3. Architecture

```
┌─────────────────────────────────┐
│ picolm.c                       │
│ CLI + Generation Loop           │  (273 lines)
└──────┬──────────────┬───────────┘
       │              │
┌──────┴────────┐ ┌───┴───────────────┐
│ model.h/c     │ │ sampler.h/c       │
│ GGUF Parser   │ │ Temperature +     │  (146+833 / 19+100 lines)
│ mmap Layer    │ │ Top-p Sampling    │
│ Streaming     │ └──────────┬────────┘
│ Forward Pass  │            │
│ KV Cache I/O  │ ┌──────────┴────────┐
└───┬────┬──────┘ │ grammar.h/c       │
    │    │        │ JSON Constraint   │  (64+175 lines)
    │    │        │ Logit Masking     │
┌───┴────┴──┐     └───────────────────┘
│ tensor.h/c │
│ matmul     │  (44+298 lines)
│ rmsnorm    │
│ softmax    │
│ rope       │
│ silu       │
│ threading  │
└─────┬──────┘
      │
┌─────┴──────┐  ┌────────────────┐
│ quant.h/c  │  │ tokenizer.h/c  │
│ Q4_K, Q6_K │  │ BPE Encode     │  (140+534 / 32+200 lines)
│ Q3_K, Q2_K │  │ Decode         │
│ FP16, F32  │  │ Vocab Lookup   │
│ NEON + SSE │  └────────────────┘
│ Fused Dots │
└────────────┘
```

## 4. Core Features

### 4.1 Memory-Mapped Inference (mmap trick)
- Model file is memory-mapped (mmap on Linux/macOS, MapViewOfFile on Windows)
- Weight pointers point directly into mapped file — no copying
- During forward pass, layers accessed sequentially
- OS automatically pages in needed weights, evicts old ones
- `madvise(MADV_SEQUENTIAL)` hints access pattern
- **638MB model runs on device with 256MB RAM** (only ~30MB in physical memory)

### 4.2 Quantization
- Q4_K_M (best quality/size balance): 1.1B params × ~0.56 bytes = 638 MB
  - vs Original: 1.1B × 4 bytes = 4.4 GB
- Supported: Q4_K, Q6_K, Q3_K, Q2_K, FP16, F32
- Quality loss minimal (6-bit scales per 32-weight sub-block)

### 4.3 9 Performance Optimizations
1. **ARM NEON SIMD**: 4-wide float vector ops (vmovl_u8, vld2q_f32, etc.)
2. **x86 SSE2 SIMD**: Auto-detected, 4-wide __m128 operations
3. **FP16 KV Cache**: Halves cache memory (88MB → 44MB), software fp32↔fp16
4. **Pre-computed RoPE Tables**: Lookup vs sinf()/cosf()/powf() per token
5. **Flash Attention (Online Softmax)**: Single-pass, eliminates O(seq_len) buffer
6. **Fused Dequantize + Dot Product**: ~50% less memory traffic for matmul
7. **Multi-threaded Matrix Multiply**: pthreads, scales to ~8 cores
8. **Grammar-Constrained JSON**: `--json` mode, pre-analyzed vocab → valid JSON
9. **KV Cache Persistence**: `--cache file.kvc`, 74% latency reduction for repeated prompts

### 4.4 Grouped-Query Attention (GQA)
- 32 query heads, 4 KV heads → 8x reduction in KV cache

### 4.5 Supported Models
- Any LLaMA-architecture model in GGUF format
- Recommended: TinyLlama 1.1B Q4_K_M (fits 256MB+ RAM)
- Can run Llama 2 7B (needs ~1.4GB RAM for KV cache)

### 4.6 Platform Support
```
make native      # x86/ARM auto-detect (recommended)
make pi          # Raspberry Pi 3/4/5 (64-bit ARM + NEON)
make pi-arm32    # Pi Zero / Pi 1 (32-bit ARM)
make cross-pi    # Cross-compile for Pi from x86
make riscv       # RISC-V (Sipeed LicheeRV, etc.)
make static      # Static binary for single-file deploy
make debug       # Debug build
build.bat        # Windows MSVC
```

### 4.7 Integration với PicoClaw
- PicoLM được thiết kế làm "local brain" cho PicoClaw (Go agent)
- PicoClaw spawns PicoLM as subprocess
- Communication: stdin → prompt, stdout → response
- `--json` grammar mode → reliable tool calling

## 5. Roadmap của PicoLM

- [ ] AVX2/AVX-512 kernels (2-4x speed trên modern CPUs)
- [ ] Speculative decoding với draft model
- [ ] Context sliding window (infinite generation)
- [ ] Weight pruning (further memory reduction)
- [ ] Continuous batching (server mode)
- [ ] Mistral / Phi architecture support

## 6. Key Takeaways cho BizClaw

1. **Rewrite C → Rust**: ~2,500 lines → safe, modern, maintainable
2. **mmap pattern**: Giữ nguyên, Rust có `memmap2` crate
3. **SIMD optimization**: Rust `std::arch` với `#[cfg(target_arch)]`
4. **GGUF parser**: Rewrite, giữ API compatible
5. **Quantization kernels**: Port NEON/SSE2 ops sang Rust intrinsics
6. **stdin/stdout protocol**: Cải thiện thành native library call
7. **Grammar JSON mode**: Essential for tool calling
8. **KV Cache persistence**: Keep for performance
9. **Integration model**: Thay stdin/stdout → in-process FFI hoặc shared library

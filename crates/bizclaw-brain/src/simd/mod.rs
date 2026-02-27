//! SIMD acceleration module.
//!
//! Platform-specific SIMD intrinsics for accelerated math operations.
//! Falls back to scalar Rust code when SIMD is unavailable.
//!
//! Supported architectures:
//! - ARM64 (aarch64): NEON — 128-bit vectors (Raspberry Pi 4/5, Apple Silicon)
//! - x86_64 + SSE2: 128-bit vectors (all x86_64 CPUs)
//! - x86_64 + AVX2: 256-bit vectors (Intel Haswell+, AMD Zen+)

pub mod avx2;
pub mod neon;
pub mod sse2;

/// Accelerated dot product — dispatches to best SIMD available.
pub fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());

    #[cfg(target_arch = "aarch64")]
    {
        neon::dot_product_neon(a, b)
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        avx2::dot_product_avx2(a, b)
    }

    #[cfg(all(target_arch = "x86_64", not(target_feature = "avx2")))]
    {
        sse2::dot_product_sse2(a, b)
    }

    // Fallback
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        crate::tensor::dot_product(a, b)
    }
}

/// Accelerated matmul using SIMD dot product.
/// output[rows] = mat[rows x cols] @ vec[cols]
pub fn matmul_simd(output: &mut [f32], mat: &[f32], vec: &[f32], rows: usize, cols: usize) {
    for i in 0..rows {
        let row = &mat[i * cols..(i + 1) * cols];
        output[i] = dot_product_simd(row, vec);
    }
}

/// Accelerated RMSNorm using SIMD reductions.
pub fn rmsnorm_simd(output: &mut [f32], input: &[f32], weight: &[f32], eps: f32) {
    let n = input.len();

    // Sum of squares using SIMD
    let ss = dot_product_simd(input, input) / n as f32;
    let inv_rms = 1.0 / (ss + eps).sqrt();

    for i in 0..n {
        output[i] = input[i] * inv_rms * weight[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_product_simd() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let result = dot_product_simd(&a, &b);
        assert!((result - 36.0).abs() < 1e-4, "got {result}");
    }

    #[test]
    fn test_matmul_simd() {
        let mat = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let vec_in = vec![1.0, 1.0, 1.0];
        let mut output = vec![0.0; 2];
        matmul_simd(&mut output, &mat, &vec_in, 2, 3);
        assert!((output[0] - 6.0).abs() < 1e-4);
        assert!((output[1] - 15.0).abs() < 1e-4);
    }
}

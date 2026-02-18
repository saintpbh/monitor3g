/// Alpha Blending Module
///
/// Blends an overlay image onto a background image using alpha compositing.
/// Formula: Out = Overlay * Alpha + Background * (1 - Alpha)
///
/// Assumes BGRA format for both buffers.

pub fn blend_frames(
    bg: &mut [u8],
    overlay: &[u8],
    width: u32,
    height: u32,
) {
    let len = (width * height * 4) as usize;
    if bg.len() < len || overlay.len() < len {
        return;
    }

    // Process 4 bytes at a time: B, G, R, A
    // Optimized loop could use SIMD, but for now standard Rust loop
    for i in (0..len).step_by(4) {
        let ov_a = overlay[i + 3] as u32;

        // Optimization: If overlay is fully transparent, skip
        if ov_a == 0 {
            continue;
        }

        // Optimization: If overlay is fully opaque, overwrite
        if ov_a == 255 {
            bg[i] = overlay[i];         // B
            bg[i+1] = overlay[i+1];     // G
            bg[i+2] = overlay[i+2];     // R
            bg[i+3] = 255;              // A (Background becomes opaque)
            continue;
        }

        // Alpha Blending
        // Using u32 for precision to avoid overflow before division
        // Formula: Result = (Foreground * Alpha + Background * (255 - Alpha)) / 255
        
        let inv_a = 255 - ov_a;
        
        let bg_b = bg[i] as u32;
        let bg_g = bg[i+1] as u32;
        let bg_r = bg[i+2] as u32;

        let ov_b = overlay[i] as u32;
        let ov_g = overlay[i+1] as u32;
        let ov_r = overlay[i+2] as u32;

        bg[i] = ((ov_b * ov_a + bg_b * inv_a) / 255) as u8;
        bg[i+1] = ((ov_g * ov_a + bg_g * inv_a) / 255) as u8;
        bg[i+2] = ((ov_r * ov_a + bg_r * inv_a) / 255) as u8;
        bg[i+3] = 255; // Output is always fully opaque
    }
}

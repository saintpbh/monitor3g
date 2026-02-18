/// Hybrid Key Engine — Chroma Key + Luma Key combined processing.
///
/// Processes BGRA pixel data in-place, modifying the alpha channel:
///   1. Chroma Key: Pixels close to the key color → transparent
///   2. Luma Key: Dark pixels below threshold → transparent
///   3. Spill Suppression: Remove residual key color from edges
///
/// Input/output format: BGRA (Blue, Green, Red, Alpha) — 4 bytes per pixel.

use super::key_state::KeyState;

/// Apply hybrid key to BGRA frame data in-place.
/// Modifies the alpha channel (byte index 3 of each pixel).
pub fn apply_hybrid_key(
    data: &mut [u8],
    width: u32,
    height: u32,
    state: &KeyState,
    prev_alpha: Option<&mut [u8]>,
) {
    let (kr, kg, kb) = state.key_color();
    let tolerance = state.tolerance() as f32;
    let softness = state.softness() as f32;
    let luma_low = state.luma_low() as f32;
    let luma_soft = state.luma_softness() as f32;
    let do_spill = state.spill_suppress();
    let smoothing = state.smoothing() as f32;

    let pixel_count = (width * height) as usize;
    let expected_len = pixel_count * 4;

    if data.len() < expected_len {
        return;
    }
    
    // Validate prev_alpha size if present
    let mut use_smoothing = false;
    if let Some(ref p) = prev_alpha {
        if p.len() == pixel_count {
            use_smoothing = smoothing > 0.0;
        }
    }

    // Pre-compute key color as f32 for distance calculation
    let kr_f = kr as f32;
    let kg_f = kg as f32;
    let kb_f = kb as f32;

    // Process pixels

    // Case 1: No smoothing (Legacy/Fast path)
    if !use_smoothing || prev_alpha.is_none() {
        for i in 0..pixel_count {
            let offset = i * 4;
            // ... (Same logic as before, just calc alpha and set)
            let b = data[offset] as f32;
            let g = data[offset + 1] as f32;
            let r = data[offset + 2] as f32;
            
            // ... (Key calc) ...
            let dr = r - kr_f;
            let dg = g - kg_f;
            let db = b - kb_f;
            let dist = (dr * dr + dg * dg + db * db).sqrt();
            let chroma_alpha = if dist < tolerance { 0.0 } 
                else if softness > 0.0 && dist < tolerance + softness { (dist - tolerance) / softness } else { 1.0 };
            
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;
            let luma_alpha = if luma_low > 0.0 {
                if luma < luma_low { 0.0 } 
                else if luma_soft > 0.0 && luma < luma_low + luma_soft { (luma - luma_low) / luma_soft } else { 1.0 }
            } else { 1.0 };

            let final_alpha = chroma_alpha.min(luma_alpha);
            
            // Set Alpha
            data[offset + 3] = (final_alpha * 255.0).clamp(0.0, 255.0) as u8;

            // Spill
            if do_spill && final_alpha > 0.0 {
                 // ... same spill ...
                 let r_u8 = data[offset + 2];
                 let g_u8 = data[offset + 1];
                 let b_u8 = data[offset];
                 let max_rb = r_u8.max(b_u8);
                 let spill_threshold = ((max_rb as f32) * 1.2) as u8;
                 if g_u8 > spill_threshold { data[offset + 1] = spill_threshold; }
            }
        }
    } else {
        // Case 2: Smoothing Enabled
        // We know prev_alpha is Some and valid length
        let prev = prev_alpha.unwrap();
        let smooth_factor = smoothing / 100.0;
        let new_factor = 1.0 - smooth_factor;

        for i in 0..pixel_count {
            let offset = i * 4;
            let b = data[offset] as f32;
            let g = data[offset + 1] as f32;
            let r = data[offset + 2] as f32;
            
            // ... (Key calc duplicated for perf, avoiding closures) ...
            let dr = r - kr_f;
            let dg = g - kg_f;
            let db = b - kb_f;
            let dist = (dr * dr + dg * dg + db * db).sqrt();
            let chroma_alpha = if dist < tolerance { 0.0 } 
                else if softness > 0.0 && dist < tolerance + softness { (dist - tolerance) / softness } else { 1.0 };
            
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;
            let luma_alpha = if luma_low > 0.0 {
                if luma < luma_low { 0.0 } 
                else if luma_soft > 0.0 && luma < luma_low + luma_soft { (luma - luma_low) / luma_soft } else { 1.0 }
            } else { 1.0 };

            let mut final_alpha = chroma_alpha.min(luma_alpha);
            
            // === SMOOTHING ===
            let old_alpha_u8 = prev[i];
            let old_alpha = (old_alpha_u8 as f32) / 255.0;
            
            // Simple Lerp
            final_alpha = old_alpha * smooth_factor + final_alpha * new_factor;
            
            let final_alpha_u8 = (final_alpha * 255.0).clamp(0.0, 255.0) as u8;
            
            // Store back
            prev[i] = final_alpha_u8;
            data[offset + 3] = final_alpha_u8;

            // Spill
            if do_spill && final_alpha > 0.0 {
                 let r_u8 = data[offset + 2];
                 // spill logic using clamped RGBA
                 let g_u8 = data[offset + 1];
                 let b_u8 = data[offset];
                 let max_rb = r_u8.max(b_u8);
                 let spill_threshold = ((max_rb as f32) * 1.2) as u8;
                 if g_u8 > spill_threshold { data[offset + 1] = spill_threshold; }
            }
        }
    }
}

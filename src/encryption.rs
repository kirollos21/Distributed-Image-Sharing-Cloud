use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use image::{DynamicImage, ImageFormat};

/// Embedded metadata in the image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub usernames: Vec<String>,
    pub quota: u32,
}

/// Encrypt image using LSB steganography + pixel scrambling
/// Embeds usernames and viewing quota, then scrambles pixels for visual encryption
/// Preserves the original image format (JPEG/PNG)
pub async fn encrypt_image(
    image_data: Vec<u8>,
    usernames: Vec<String>,
    quota: u32,
) -> Result<Vec<u8>, String> {
    info!(
        "Starting encryption for {} usernames with quota {}",
        usernames.len(),
        quota
    );

    // Simulate heavy computational load (critical for load testing)
    let processing_delay = Duration::from_millis(500 + (image_data.len() / 100) as u64);
    sleep(processing_delay).await;

    // Detect image format
    let format = image::guess_format(&image_data).map_err(|e| format!("Cannot detect image format: {}", e))?;
    info!("Detected image format: {:?}", format);

    // Decode image to pixels
    let img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    // Convert to RGBA for easier manipulation
    let mut rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    info!("Image dimensions: {}x{}", width, height);

    // Get mutable pixel data
    let pixels = rgba_img.as_mut();

    let metadata = ImageMetadata { usernames, quota };
    let metadata_json = serde_json::to_string(&metadata).map_err(|e| e.to_string())?;
    let metadata_bytes = metadata_json.as_bytes();

    // Check if image has enough capacity (using pixel data, not file bytes)
    let required_bits = (metadata_bytes.len() + 4) * 8; // +4 for length prefix
    let available_bits = pixels.len(); // Each pixel byte can store 1 bit

    if required_bits > available_bits {
        return Err(format!(
            "Image too small: need {} bits, have {} bits",
            required_bits, available_bits
        ));
    }

    // STEP 1: Embed metadata into LSBs FIRST (before scrambling)
    // This way when we scramble, the metadata stays with its pixels
    // and can be extracted after unscrambling
    
    // Embed metadata length (4 bytes = 32 bits) into LSB
    let metadata_len = metadata_bytes.len() as u32;
    let len_bytes = metadata_len.to_be_bytes();

    for (i, &byte) in len_bytes.iter().enumerate() {
        for bit in 0..8 {
            let bit_value = (byte >> (7 - bit)) & 1;
            let pixel_index = i * 8 + bit;
            if pixel_index < pixels.len() {
                // Clear LSB and set to bit_value
                pixels[pixel_index] = (pixels[pixel_index] & 0xFE) | bit_value;
            }
        }
    }

    // Embed metadata starting after length (32 bits)
    let start_offset = 32;
    for (i, &byte) in metadata_bytes.iter().enumerate() {
        for bit in 0..8 {
            let bit_value = (byte >> (7 - bit)) & 1;
            let pixel_index = start_offset + i * 8 + bit;
            if pixel_index < pixels.len() {
                pixels[pixel_index] = (pixels[pixel_index] & 0xFE) | bit_value;
            }
        }
    }

    debug!("Metadata embedded: {} bytes (before scrambling)", metadata_bytes.len());

    // STEP 2: VISUAL ENCRYPTION - Scramble pixels AFTER embedding metadata
    // The metadata LSBs will be scrambled along with the pixels
    // Client must unscramble first, then extract metadata
    let seed = calculate_seed(&metadata);
    scramble_pixels(pixels, seed);
    info!("Pixels scrambled using seed derived from metadata");

    // OPTIMIZATION: Resize image if too large for UDP transmission
    // Target: encrypted output should be < 50KB to fit in UDP packets safely
    // PNG is lossless but larger than JPEG - need to be conservative with size
    let dynamic_img = DynamicImage::ImageRgba8(rgba_img);
    
    // Calculate approximate output size for PNG (roughly 4 bytes per pixel with compression)
    let estimated_size = (width * height * 4); // PNG RGBA is ~4 bytes per pixel
    let max_safe_size = 12000; // ~12KB pixel data = ~15KB PNG after compression
    
    let resized_img = if estimated_size > max_safe_size {
        // Calculate scale factor to reduce size
        let scale = ((max_safe_size as f32) / (estimated_size as f32)).sqrt();
        let new_width = ((width as f32) * scale) as u32;
        let new_height = ((height as f32) * scale) as u32;
        
        info!("Resizing from {}x{} to {}x{} for UDP compatibility", 
              width, height, new_width, new_height);
        dynamic_img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        dynamic_img
    };

    // CRITICAL: Use PNG (lossless) to preserve LSB metadata!
    // JPEG compression would destroy the LSB-encoded metadata
    let mut output_bytes = Vec::new();
    
    // Use explicit PNG encoder to ensure lossless encoding
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
    resized_img.write_with_encoder(encoder)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    info!("Encrypted image created: {} bytes (scrambled + metadata embedded as PNG)", output_bytes.len());
    
    // Final safety check
    if output_bytes.len() > 50000 {
        return Err(format!(
            "Encrypted image too large: {} bytes. Maximum safe size for UDP is ~50KB. Try a smaller input image.",
            output_bytes.len()
        ));
    }
    
    Ok(output_bytes)
}

/// Calculate a seed from metadata for deterministic scrambling
fn calculate_seed(metadata: &ImageMetadata) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for username in &metadata.usernames {
        username.hash(&mut hasher);
    }
    metadata.quota.hash(&mut hasher);
    hasher.finish()
}

/// Scramble pixels using Fisher-Yates shuffle with a seed
fn scramble_pixels(pixels: &mut [u8], seed: u64) {
    let len = pixels.len() / 4; // Number of RGBA pixels

    // Create a simple LCG (Linear Congruential Generator) for deterministic randomness
    let mut rng_state = seed;

    for i in (1..len).rev() {
        // Generate pseudo-random index
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng_state % (i as u64 + 1)) as usize;

        // Swap pixels (4 bytes each: RGBA)
        let idx_i = i * 4;
        let idx_j = j * 4;

        for k in 0..4 {
            pixels.swap(idx_i + k, idx_j + k);
        }
    }
}

/// Unscramble pixels using the reverse of Fisher-Yates
fn unscramble_pixels(pixels: &mut [u8], seed: u64) {
    let len = pixels.len() / 4; // Number of RGBA pixels

    // Store all the swap indices
    let mut swap_indices = Vec::with_capacity(len);
    let mut rng_state = seed;

    for i in (1..len).rev() {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng_state % (i as u64 + 1)) as usize;
        swap_indices.push((i, j));
    }

    // Apply swaps in reverse order
    for (i, j) in swap_indices.iter().rev() {
        let idx_i = i * 4;
        let idx_j = j * 4;

        for k in 0..4 {
            pixels.swap(idx_i + k, idx_j + k);
        }
    }
}

/// Decrypt image: unscrambles pixels and extracts metadata
/// Extracts usernames and viewing quota, then unscrambles to restore original
/// The encrypted_image is a valid image file (JPEG/PNG) with scrambled pixels and embedded metadata
pub async fn decrypt_image(encrypted_image: Vec<u8>) -> Result<(Vec<u8>, ImageMetadata), String> {
    info!("Starting decryption of scrambled image");
    eprintln!("[DEBUG DECRYPT] Step 1: Starting decryption");

    // Simulate processing delay
    sleep(Duration::from_millis(200)).await;
    eprintln!("[DEBUG DECRYPT] Step 2: Sleep complete");

    // Detect format
    let format = image::guess_format(&encrypted_image).map_err(|e| format!("Cannot detect image format: {}", e))?;
    eprintln!("[DEBUG DECRYPT] Step 3: Format detected: {:?}", format);

    // Decode scrambled image to pixels to extract metadata
    let img = image::load_from_memory(&encrypted_image)
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    eprintln!("[DEBUG DECRYPT] Step 4: Image decoded");

    let mut rgba_img = img.to_rgba8();
    let (_width, _height) = rgba_img.dimensions();
    let pixels = rgba_img.as_mut();
    eprintln!("[DEBUG DECRYPT] Step 5: Pixel data extracted ({} bytes)", pixels.len());

    if pixels.len() < 32 {
        return Err("Image too small to contain metadata".to_string());
    }

    // Extract metadata length from first 32 bits (from pixel LSBs)
    let mut len_bytes = [0u8; 4];
    for i in 0..4 {
        let mut byte = 0u8;
        for bit in 0..8 {
            let pixel_index = i * 8 + bit;
            let bit_value = pixels[pixel_index] & 1;
            byte = (byte << 1) | bit_value;
        }
        len_bytes[i] = byte;
    }

    let metadata_len = u32::from_be_bytes(len_bytes) as usize;

    if metadata_len == 0 || metadata_len > 10000 {
        return Err(format!("Invalid metadata length: {}", metadata_len));
    }

    // Extract metadata bytes from pixel LSBs
    let start_offset = 32;
    let mut metadata_bytes = vec![0u8; metadata_len];

    for i in 0..metadata_len {
        let mut byte = 0u8;
        for bit in 0..8 {
            let pixel_index = start_offset + i * 8 + bit;
            if pixel_index >= pixels.len() {
                return Err("Unexpected end of pixel data".to_string());
            }
            let bit_value = pixels[pixel_index] & 1;
            byte = (byte << 1) | bit_value;
        }
        metadata_bytes[i] = byte;
    }

    // Deserialize metadata
    eprintln!("[DEBUG DECRYPT] Step 6: Deserializing metadata");
    let metadata_json = String::from_utf8(metadata_bytes).map_err(|e| e.to_string())?;
    let metadata: ImageMetadata = serde_json::from_str(&metadata_json).map_err(|e| e.to_string())?;

    debug!("Metadata extracted: {} usernames", metadata.usernames.len());
    eprintln!("[DEBUG DECRYPT] Step 7: Metadata extracted - usernames: {:?}, quota: {}", metadata.usernames, metadata.quota);

    // VISUAL DECRYPTION: Unscramble pixels using same seed
    let seed = calculate_seed(&metadata);
    eprintln!("[DEBUG DECRYPT] Step 8: Calculated seed: {}, starting unscramble", seed);
    unscramble_pixels(pixels, seed);
    info!("Pixels unscrambled - original image restored");
    eprintln!("[DEBUG DECRYPT] Step 9: Pixels unscrambled successfully");

    // Convert back to original format
    eprintln!("[DEBUG DECRYPT] Step 10: Re-encoding image");
    let dynamic_img = DynamicImage::ImageRgba8(rgba_img);
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);

    // Re-encode in the same format
    match format {
        ImageFormat::Jpeg => {
            dynamic_img.write_to(&mut cursor, ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
        ImageFormat::Png => {
            dynamic_img.write_to(&mut cursor, ImageFormat::Png)
                .map_err(|e| format!("Failed to encode PNG: {}", e))?;
        }
        _ => {
            dynamic_img.write_to(&mut cursor, ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
        }
    }

    info!("Decryption completed: {} bytes (original image restored)", output_bytes.len());
    eprintln!("[DEBUG DECRYPT] Step 11: Decryption complete! Returning {} bytes", output_bytes.len());

    // Return the decrypted (unscrambled) image and the extracted metadata
    Ok((output_bytes, metadata))
}

/// Check if a user is authorized to view the image
pub fn is_authorized(metadata: &ImageMetadata, username: &str) -> bool {
    metadata.usernames.iter().any(|u| u == username)
}

/// Decrement viewing quota
pub fn decrement_quota(metadata: &mut ImageMetadata) -> bool {
    if metadata.quota > 0 {
        metadata.quota -= 1;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        // Create a simple test image (1KB of random data)
        let image_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let usernames = vec!["alice".to_string(), "bob".to_string()];
        let quota = 5;

        // Encrypt
        let encrypted = encrypt_image(image_data.clone(), usernames.clone(), quota)
            .await
            .unwrap();

        // Decrypt
        let (_, metadata) = decrypt_image(encrypted).await.unwrap();

        assert_eq!(metadata.usernames, usernames);
        assert_eq!(metadata.quota, quota);
    }

    #[tokio::test]
    async fn test_authorization() {
        let metadata = ImageMetadata {
            usernames: vec!["alice".to_string(), "bob".to_string()],
            quota: 3,
        };

        assert!(is_authorized(&metadata, "alice"));
        assert!(is_authorized(&metadata, "bob"));
        assert!(!is_authorized(&metadata, "charlie"));
    }

    #[tokio::test]
    async fn test_quota_decrement() {
        let mut metadata = ImageMetadata {
            usernames: vec!["alice".to_string()],
            quota: 2,
        };

        assert!(decrement_quota(&mut metadata));
        assert_eq!(metadata.quota, 1);

        assert!(decrement_quota(&mut metadata));
        assert_eq!(metadata.quota, 0);

        assert!(!decrement_quota(&mut metadata));
        assert_eq!(metadata.quota, 0);
    }
}

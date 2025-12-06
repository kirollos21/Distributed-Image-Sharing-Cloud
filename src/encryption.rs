use log::info;
use serde::{Deserialize, Serialize};
// use std::time::Duration;  // Commented out - no longer using artificial delays
// use tokio::time::sleep;   // Commented out - no longer using artificial delays
use image::{DynamicImage, GenericImageView};

/// Embedded metadata in the image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub usernames: Vec<String>,
    pub quota: u8,
    #[serde(default)]
    pub viewed: u8,  // Counter for how many times the image has been viewed (starts at 0)
}

/// Encrypt image by hiding it inside a cover image using LSB steganography
/// The cover image becomes the "encryption key" - the encrypted result looks like the cover
/// Embeds: [metadata_len][metadata][original_image_len][original_image_data] all in LSBs
pub async fn encrypt_image(
    image_data: Vec<u8>,
    usernames: Vec<String>,
    quota: u8,
) -> Result<Vec<u8>, String> {
    info!(
        "Starting encryption for {} usernames with quota {}",
        usernames.len(),
        quota
    );

    // Decode the original image to get its dimensions
    let original_img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Failed to decode original image: {}", e))?;
    let (orig_width, orig_height) = original_img.dimensions();
    info!("Original image dimensions: {}x{}", orig_width, orig_height);

    // Load the encryption key image as the cover
    let cover_img = load_cover_image()?;
    let (cover_width, cover_height) = (cover_img.width(), cover_img.height());
    info!("Loaded cover image (encryption key): {}x{}", cover_width, cover_height);
    let mut cover_img = cover_img;

    // Get mutable pixel data from cover image
    let pixels = cover_img.as_mut();
    let available_bits = pixels.len(); // Each byte can hold 1 bit in LSB

    // Prepare metadata (viewed starts at 0 for new images)
    let metadata = ImageMetadata { usernames, quota, viewed: 0 };
    let metadata_json = serde_json::to_string(&metadata).map_err(|e| e.to_string())?;
    let metadata_bytes = metadata_json.as_bytes();

    // Calculate total bits needed
    let metadata_header_bits = 32; // 4 bytes for metadata length
    let metadata_bits = metadata_bytes.len() * 8;
    let image_header_bits = 32; // 4 bytes for original image length
    let image_bits = image_data.len() * 8;
    let total_bits = metadata_header_bits + metadata_bits + image_header_bits + image_bits;

    info!("Capacity check: need {} bits, have {} bits", total_bits, available_bits);

    if total_bits > available_bits {
        return Err(format!(
            "Cover image too small: need {} bits, have {} bits",
            total_bits, available_bits
        ));
    }

    let mut bit_index = 0;

    // STEP 1: Embed metadata length (4 bytes)
    let metadata_len = metadata_bytes.len() as u32;
    embed_u32(pixels, &mut bit_index, metadata_len);

    // STEP 2: Embed metadata
    embed_bytes(pixels, &mut bit_index, metadata_bytes);
    info!("Metadata embedded: {} bytes", metadata_bytes.len());

    // STEP 3: Embed original image length (4 bytes)
    let image_len = image_data.len() as u32;
    embed_u32(pixels, &mut bit_index, image_len);

    // STEP 4: Embed original image data
    embed_bytes(pixels, &mut bit_index, &image_data);
    info!("Original image embedded: {} bytes", image_data.len());

    // Convert to DynamicImage and encode as PNG (lossless)
    let final_img = DynamicImage::ImageRgb8(cover_img);
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
    final_img.write_with_encoder(encoder)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    info!("Encrypted image created: {} bytes (looks like cover image)", output_bytes.len());
    Ok(output_bytes)
}

/// Load the encryption key image from disk
fn load_cover_image() -> Result<image::RgbImage, String> {
    use std::fs;

    // Try to load encrypction_key.jpg (note the typo in the filename)
    let key_path = "encrypction_key.jpg";

    if !std::path::Path::new(key_path).exists() {
        return Err(format!("Encryption key image not found: {}", key_path));
    }

    let key_data = fs::read(key_path)
        .map_err(|e| format!("Failed to read encryption key image: {}", e))?;

    let key_img = image::load_from_memory(&key_data)
        .map_err(|e| format!("Failed to decode encryption key image: {}", e))?;

    // Convert to RGB image
    Ok(key_img.to_rgb8())
}

/// Embed a u32 value into LSBs
fn embed_u32(pixels: &mut [u8], bit_index: &mut usize, value: u32) {
    let bytes = value.to_be_bytes();
    embed_bytes(pixels, bit_index, &bytes);
}

/// Embed bytes into LSBs
fn embed_bytes(pixels: &mut [u8], bit_index: &mut usize, data: &[u8]) {
    for &byte in data {
        for bit_pos in (0..8).rev() {
            let bit_value = (byte >> bit_pos) & 1;
            pixels[*bit_index] = (pixels[*bit_index] & 0xFE) | bit_value;
            *bit_index += 1;
        }
    }
}

/// Extract a u32 value from LSBs
fn extract_u32(pixels: &[u8], bit_index: &mut usize) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    extract_bytes(pixels, bit_index, &mut bytes)?;
    Ok(u32::from_be_bytes(bytes))
}

/// Extract bytes from LSBs
fn extract_bytes(pixels: &[u8], bit_index: &mut usize, output: &mut [u8]) -> Result<(), String> {
    for byte_out in output.iter_mut() {
        let mut byte = 0u8;
        for _ in 0..8 {
            if *bit_index >= pixels.len() {
                return Err("Unexpected end of pixel data".to_string());
            }
            let bit_value = pixels[*bit_index] & 1;
            byte = (byte << 1) | bit_value;
            *bit_index += 1;
        }
        *byte_out = byte;
    }
    Ok(())
}


/// Decrypt image: extracts hidden image from cover image using LSB steganography
/// Extracts: [metadata_len][metadata][original_image_len][original_image_data] from LSBs
/// Returns the original hidden image and the metadata
pub async fn decrypt_image(encrypted_image: Vec<u8>) -> Result<(Vec<u8>, ImageMetadata), String> {
    info!("Starting decryption - extracting hidden image from cover");

    // Decode the cover image (encryption key)
    let img = image::load_from_memory(&encrypted_image)
        .map_err(|e| format!("Failed to decode encrypted image: {}", e))?;

    let rgb_img = img.to_rgb8();
    let (_width, _height) = rgb_img.dimensions();
    let pixels = rgb_img.as_raw();

    if pixels.len() < 64 {
        return Err("Image too small to contain hidden data".to_string());
    }

    let mut bit_index = 0;

    // STEP 1: Extract metadata length (4 bytes)
    let metadata_len = extract_u32(pixels, &mut bit_index)? as usize;

    if metadata_len == 0 || metadata_len > 10000 {
        return Err(format!("Invalid metadata length: {}", metadata_len));
    }

    info!("Metadata length: {} bytes", metadata_len);

    // STEP 2: Extract metadata
    let mut metadata_bytes = vec![0u8; metadata_len];
    extract_bytes(pixels, &mut bit_index, &mut metadata_bytes)?;

    let metadata_json = String::from_utf8(metadata_bytes)
        .map_err(|e| format!("Invalid metadata UTF-8: {}", e))?;
    let metadata: ImageMetadata = serde_json::from_str(&metadata_json)
        .map_err(|e| format!("Invalid metadata JSON: {}", e))?;

    info!("Metadata extracted: {} usernames, quota: {}", metadata.usernames.len(), metadata.quota);

    // STEP 3: Extract original image length (4 bytes)
    let image_len = extract_u32(pixels, &mut bit_index)? as usize;

    if image_len == 0 || image_len > 10_000_000 {
        return Err(format!("Invalid image length: {}", image_len));
    }

    info!("Original image length: {} bytes", image_len);

    // STEP 4: Extract original image data
    let mut original_image_data = vec![0u8; image_len];
    extract_bytes(pixels, &mut bit_index, &mut original_image_data)?;

    info!("Decryption completed: extracted {} bytes (original image)", original_image_data.len());

    // Return the original hidden image and metadata
    Ok((original_image_data, metadata))
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

/// Check if the image can still be viewed (viewed count < quota)
pub fn can_view(metadata: &ImageMetadata) -> bool {
    metadata.viewed < metadata.quota
}

/// Get remaining views for an image
pub fn remaining_views(metadata: &ImageMetadata) -> u8 {
    metadata.quota.saturating_sub(metadata.viewed)
}

/// Update the view count in an encrypted image file
/// Returns: Ok(Some(metadata)) if view was counted, Ok(None) if quota exhausted, Err on failure
pub async fn update_view_count(image_data: &[u8]) -> Result<(Vec<u8>, ImageMetadata, bool), String> {
    info!("Updating view count in encrypted image");

    // Decode the cover image
    let img = image::load_from_memory(image_data)
        .map_err(|e| format!("Failed to decode encrypted image: {}", e))?;

    let rgb_img = img.to_rgb8();
    let pixels = rgb_img.as_raw();

    if pixels.len() < 64 {
        return Err("Image too small to contain hidden data".to_string());
    }

    let mut bit_index = 0;

    // STEP 1: Extract metadata length (4 bytes)
    let metadata_len = extract_u32(pixels, &mut bit_index)? as usize;

    if metadata_len == 0 || metadata_len > 10000 {
        return Err(format!("Invalid metadata length: {}", metadata_len));
    }

    // STEP 2: Extract metadata
    let mut metadata_bytes = vec![0u8; metadata_len];
    extract_bytes(pixels, &mut bit_index, &mut metadata_bytes)?;

    let metadata_json = String::from_utf8(metadata_bytes)
        .map_err(|e| format!("Invalid metadata UTF-8: {}", e))?;
    let mut metadata: ImageMetadata = serde_json::from_str(&metadata_json)
        .map_err(|e| format!("Invalid metadata JSON: {}", e))?;

    // Check if quota is exhausted
    if metadata.viewed >= metadata.quota {
        info!("Quota exhausted: viewed {} times, quota is {}", metadata.viewed, metadata.quota);
        return Ok((image_data.to_vec(), metadata, false));
    }

    // STEP 3: Extract original image length (4 bytes)
    let image_len = extract_u32(pixels, &mut bit_index)? as usize;

    if image_len == 0 || image_len > 10_000_000 {
        return Err(format!("Invalid image length: {}", image_len));
    }

    // STEP 4: Extract original image data
    let mut original_image_data = vec![0u8; image_len];
    extract_bytes(pixels, &mut bit_index, &mut original_image_data)?;

    // Increment view count
    metadata.viewed += 1;
    info!("View count updated: {} / {}", metadata.viewed, metadata.quota);

    // Re-embed with updated metadata
    let updated_encrypted = re_embed_metadata(&rgb_img, &metadata, &original_image_data)?;

    Ok((updated_encrypted, metadata, true))
}

/// Re-embed metadata into an existing cover image with the same original image data
fn re_embed_metadata(
    cover_img: &image::RgbImage,
    metadata: &ImageMetadata,
    original_image_data: &[u8],
) -> Result<Vec<u8>, String> {
    let mut cover_img = cover_img.clone();
    let pixels = cover_img.as_mut();

    // Prepare new metadata
    let metadata_json = serde_json::to_string(metadata).map_err(|e| e.to_string())?;
    let metadata_bytes = metadata_json.as_bytes();

    let mut bit_index = 0;

    // STEP 1: Embed metadata length (4 bytes)
    let metadata_len = metadata_bytes.len() as u32;
    embed_u32(pixels, &mut bit_index, metadata_len);

    // STEP 2: Embed metadata
    embed_bytes(pixels, &mut bit_index, metadata_bytes);

    // STEP 3: Embed original image length (4 bytes)
    let image_len = original_image_data.len() as u32;
    embed_u32(pixels, &mut bit_index, image_len);

    // STEP 4: Embed original image data
    embed_bytes(pixels, &mut bit_index, original_image_data);

    // Convert to DynamicImage and encode as PNG (lossless)
    let final_img = DynamicImage::ImageRgb8(cover_img);
    let mut output_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut output_bytes);
    let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
    final_img.write_with_encoder(encoder)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    info!("Re-embedded image with updated metadata: {} bytes", output_bytes.len());
    Ok(output_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a valid PNG image for testing
    fn create_test_image() -> Vec<u8> {
        use image::{ImageBuffer, Rgb};
        
        // Create a small 10x10 RGB image
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(10, 10, |x, y| {
            Rgb([(x * 25) as u8, (y * 25) as u8, 128u8])
        });
        
        // Encode as PNG
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
        img.write_with_encoder(encoder).unwrap();
        buffer
    }

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        // Create a valid test image
        let image_data = create_test_image();
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
        assert_eq!(metadata.viewed, 0); // New images start with viewed = 0
    }

    #[tokio::test]
    async fn test_authorization() {
        let metadata = ImageMetadata {
            usernames: vec!["alice".to_string(), "bob".to_string()],
            quota: 3,
            viewed: 0,
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
            viewed: 0,
        };

        assert!(decrement_quota(&mut metadata));
        assert_eq!(metadata.quota, 1);

        assert!(decrement_quota(&mut metadata));
        assert_eq!(metadata.quota, 0);

        assert!(!decrement_quota(&mut metadata));
        assert_eq!(metadata.quota, 0);
    }

    #[tokio::test]
    async fn test_view_count() {
        // Create a valid test image
        let image_data = create_test_image();
        let usernames = vec!["alice".to_string()];
        let quota = 3;

        // Encrypt
        let encrypted = encrypt_image(image_data.clone(), usernames.clone(), quota)
            .await
            .unwrap();

        // First view - should succeed
        let (updated1, meta1, success1) = update_view_count(&encrypted).await.unwrap();
        assert!(success1);
        assert_eq!(meta1.viewed, 1);
        assert!(can_view(&meta1)); // 1 < 3, can still view

        // Second view - should succeed
        let (updated2, meta2, success2) = update_view_count(&updated1).await.unwrap();
        assert!(success2);
        assert_eq!(meta2.viewed, 2);
        assert!(can_view(&meta2)); // 2 < 3, can still view

        // Third view - should succeed (last view)
        let (updated3, meta3, success3) = update_view_count(&updated2).await.unwrap();
        assert!(success3);
        assert_eq!(meta3.viewed, 3);
        assert!(!can_view(&meta3)); // 3 >= 3, cannot view anymore

        // Fourth view - should fail (quota exhausted)
        let (_updated4, meta4, success4) = update_view_count(&updated3).await.unwrap();
        assert!(!success4);
        assert_eq!(meta4.viewed, 3); // Stayed at 3
    }

    #[tokio::test]
    async fn test_remaining_views() {
        let metadata = ImageMetadata {
            usernames: vec!["alice".to_string()],
            quota: 5,
            viewed: 2,
        };
        assert_eq!(remaining_views(&metadata), 3);

        let metadata_exhausted = ImageMetadata {
            usernames: vec!["alice".to_string()],
            quota: 5,
            viewed: 5,
        };
        assert_eq!(remaining_views(&metadata_exhausted), 0);
    }
}

use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

/// Embedded metadata in the image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub usernames: Vec<String>,
    pub quota: u32,
}

/// Encrypt image using LSB steganography
/// Embeds usernames and viewing quota into the image data
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

    let metadata = ImageMetadata { usernames, quota };
    let metadata_json = serde_json::to_string(&metadata).map_err(|e| e.to_string())?;
    let metadata_bytes = metadata_json.as_bytes();

    // Check if image has enough capacity
    let required_bits = (metadata_bytes.len() + 4) * 8; // +4 for length prefix
    let available_bits = image_data.len();

    if required_bits > available_bits {
        return Err(format!(
            "Image too small: need {} bits, have {} bits",
            required_bits, available_bits
        ));
    }

    let mut encrypted_image = image_data.clone();

    // Embed metadata length (4 bytes = 32 bits)
    let metadata_len = metadata_bytes.len() as u32;
    let len_bytes = metadata_len.to_be_bytes();

    for (i, &byte) in len_bytes.iter().enumerate() {
        for bit in 0..8 {
            let bit_value = (byte >> (7 - bit)) & 1;
            let pixel_index = i * 8 + bit;
            if pixel_index < encrypted_image.len() {
                // Clear LSB and set to bit_value
                encrypted_image[pixel_index] = (encrypted_image[pixel_index] & 0xFE) | bit_value;
            }
        }
    }

    // Embed metadata starting after length (32 bits)
    let start_offset = 32;
    for (i, &byte) in metadata_bytes.iter().enumerate() {
        for bit in 0..8 {
            let bit_value = (byte >> (7 - bit)) & 1;
            let pixel_index = start_offset + i * 8 + bit;
            if pixel_index < encrypted_image.len() {
                encrypted_image[pixel_index] = (encrypted_image[pixel_index] & 0xFE) | bit_value;
            }
        }
    }

    debug!("Encryption completed: embedded {} bytes", metadata_bytes.len());
    Ok(encrypted_image)
}

/// Decrypt image using LSB steganography
/// Extracts usernames and viewing quota from the image data
pub async fn decrypt_image(encrypted_image: Vec<u8>) -> Result<(Vec<u8>, ImageMetadata), String> {
    info!("Starting decryption of image");

    // Simulate processing delay
    sleep(Duration::from_millis(200)).await;

    if encrypted_image.len() < 32 {
        return Err("Image too small to contain metadata".to_string());
    }

    // Extract metadata length from first 32 bits
    let mut len_bytes = [0u8; 4];
    for i in 0..4 {
        let mut byte = 0u8;
        for bit in 0..8 {
            let pixel_index = i * 8 + bit;
            let bit_value = encrypted_image[pixel_index] & 1;
            byte = (byte << 1) | bit_value;
        }
        len_bytes[i] = byte;
    }

    let metadata_len = u32::from_be_bytes(len_bytes) as usize;

    if metadata_len == 0 || metadata_len > 10000 {
        return Err(format!("Invalid metadata length: {}", metadata_len));
    }

    // Extract metadata bytes
    let start_offset = 32;
    let mut metadata_bytes = vec![0u8; metadata_len];

    for i in 0..metadata_len {
        let mut byte = 0u8;
        for bit in 0..8 {
            let pixel_index = start_offset + i * 8 + bit;
            if pixel_index >= encrypted_image.len() {
                return Err("Unexpected end of image data".to_string());
            }
            let bit_value = encrypted_image[pixel_index] & 1;
            byte = (byte << 1) | bit_value;
        }
        metadata_bytes[i] = byte;
    }

    // Deserialize metadata
    let metadata_json = String::from_utf8(metadata_bytes).map_err(|e| e.to_string())?;
    let metadata: ImageMetadata = serde_json::from_str(&metadata_json).map_err(|e| e.to_string())?;

    debug!("Decryption completed: extracted {} usernames", metadata.usernames.len());
    Ok((encrypted_image.clone(), metadata))
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

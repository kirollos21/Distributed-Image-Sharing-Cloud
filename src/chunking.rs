use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use log::{debug, warn};
use base64::{Engine as _, engine::general_purpose};

/// Maximum size for a single chunk (45KB of actual data)
/// After base64 encoding (~33% overhead), becomes ~60KB
/// With JSON wrapper, stays under 65KB UDP limit
const CHUNK_SIZE: usize = 45000;

/// Timeout for incomplete chunk reassembly (30 seconds)
const REASSEMBLY_TIMEOUT: Duration = Duration::from_secs(30);

/// A chunked message that can be sent over UDP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkedMessage {
    /// Single packet - fits in one UDP datagram (base64 encoded)
    SinglePacket(String),

    /// Multi-packet chunk
    MultiPacket {
        chunk_id: String,      // Unique ID for this multi-packet message
        chunk_index: u32,      // 0-based index of this chunk
        total_chunks: u32,     // Total number of chunks
        data: String,          // Chunk data (base64 encoded)
    },
}

impl ChunkedMessage {
    /// Fragment a large message into chunks with base64 encoding
    pub fn fragment(data: Vec<u8>) -> Vec<ChunkedMessage> {
        let data_len = data.len();

        // If fits in single packet, return as base64 encoded
        if data_len <= CHUNK_SIZE {
            debug!("Message fits in single packet: {} bytes", data_len);
            let encoded = general_purpose::STANDARD.encode(&data);
            return vec![ChunkedMessage::SinglePacket(encoded)];
        }

        // Calculate number of chunks needed
        let total_chunks = ((data_len + CHUNK_SIZE - 1) / CHUNK_SIZE) as u32;
        let chunk_id = format!("{}", uuid::Uuid::new_v4());

        debug!("Fragmenting message: {} bytes into {} chunks (chunk_id: {})",
               data_len, total_chunks, chunk_id);

        // Create chunks
        let mut chunks = Vec::new();
        for chunk_index in 0..total_chunks {
            let start = (chunk_index as usize) * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE, data_len);
            let chunk_data = &data[start..end];

            // Base64 encode the chunk data
            let encoded_data = general_purpose::STANDARD.encode(chunk_data);

            chunks.push(ChunkedMessage::MultiPacket {
                chunk_id: chunk_id.clone(),
                chunk_index,
                total_chunks,
                data: encoded_data,
            });
        }

        chunks
    }
}

/// Manages reassembly of chunked messages
pub struct ChunkReassembler {
    /// Incomplete messages: chunk_id -> (received_chunks, timestamp)
    incomplete: HashMap<String, (HashMap<u32, Vec<u8>>, u32, Instant)>,
}

impl ChunkReassembler {
    pub fn new() -> Self {
        Self {
            incomplete: HashMap::new(),
        }
    }

    /// Process a chunk and return complete message if all chunks received
    pub fn process_chunk(&mut self, chunk: ChunkedMessage) -> Option<Vec<u8>> {
        match chunk {
            ChunkedMessage::SinglePacket(encoded_data) => {
                // Base64 decode the data
                match general_purpose::STANDARD.decode(&encoded_data) {
                    Ok(data) => {
                        debug!("Received single packet: {} bytes", data.len());
                        Some(data)
                    }
                    Err(e) => {
                        warn!("Failed to decode base64 single packet: {}", e);
                        None
                    }
                }
            }

            ChunkedMessage::MultiPacket {
                chunk_id,
                chunk_index,
                total_chunks,
                data: encoded_data,
            } => {
                // Base64 decode the chunk data
                let data = match general_purpose::STANDARD.decode(&encoded_data) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to decode base64 chunk data: {}", e);
                        return None;
                    }
                };

                debug!("Received chunk {}/{} for message {} ({} bytes)",
                       chunk_index + 1, total_chunks, chunk_id, data.len());

                // Get or create entry for this message
                let (chunks, expected_total, _timestamp) = self.incomplete
                    .entry(chunk_id.clone())
                    .or_insert_with(|| (HashMap::new(), total_chunks, Instant::now()));

                // Verify total_chunks matches
                if *expected_total != total_chunks {
                    warn!("Chunk total mismatch for {}: expected {}, got {}",
                          chunk_id, expected_total, total_chunks);
                    return None;
                }

                // Store this chunk
                chunks.insert(chunk_index, data.clone());

                debug!("Stored chunk {} for message {}, total stored: {}/{}",
                       chunk_index, chunk_id, chunks.len(), total_chunks);

                // Check if we have all chunks
                if chunks.len() == total_chunks as usize {
                    debug!("All chunks received for message {}, reassembling", chunk_id);

                    // Verify we have all indices
                    let mut missing_indices = Vec::new();
                    for i in 0..total_chunks {
                        if !chunks.contains_key(&i) {
                            missing_indices.push(i);
                        }
                    }

                    if !missing_indices.is_empty() {
                        warn!("Missing chunks for {}: {:?}", chunk_id, missing_indices);
                        return None;
                    }

                    // Reassemble in order
                    let mut complete_data = Vec::new();
                    for i in 0..total_chunks {
                        if let Some(chunk_data) = chunks.get(&i) {
                            complete_data.extend_from_slice(chunk_data);
                        } else {
                            warn!("Missing chunk {} for message {}", i, chunk_id);
                            return None;
                        }
                    }

                    // Remove from incomplete
                    self.incomplete.remove(&chunk_id);

                    debug!("Reassembly complete: {} bytes", complete_data.len());
                    return Some(complete_data);
                }

                None
            }
        }
    }

    /// Clean up old incomplete messages
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.incomplete.retain(|chunk_id, (_chunks, _total, timestamp)| {
            let expired = now.duration_since(*timestamp) > REASSEMBLY_TIMEOUT;
            if expired {
                warn!("Cleaning up expired incomplete message: {}", chunk_id);
            }
            !expired
        });
    }

    /// Get statistics about incomplete messages
    pub fn stats(&self) -> (usize, usize) {
        let incomplete_count = self.incomplete.len();
        let total_chunks: usize = self.incomplete.values()
            .map(|(chunks, _total, _ts)| chunks.len())
            .sum();
        (incomplete_count, total_chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_packet() {
        let data = vec![1, 2, 3, 4, 5];
        let chunks = ChunkedMessage::fragment(data.clone());

        assert_eq!(chunks.len(), 1);
        match &chunks[0] {
            ChunkedMessage::SinglePacket(encoded) => {
                // Verify it's base64 encoded
                let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
                assert_eq!(decoded, data);
            }
            _ => panic!("Expected SinglePacket"),
        }
    }

    #[test]
    fn test_multi_packet() {
        // Create data larger than CHUNK_SIZE
        let data = vec![42u8; CHUNK_SIZE * 2 + 1000];
        let chunks = ChunkedMessage::fragment(data.clone());

        assert_eq!(chunks.len(), 3); // Should be split into 3 chunks

        // Verify each chunk
        for (i, chunk) in chunks.iter().enumerate() {
            match chunk {
                ChunkedMessage::MultiPacket { chunk_index, total_chunks, data: encoded_data, .. } => {
                    assert_eq!(*chunk_index, i as u32);
                    assert_eq!(*total_chunks, 3);
                    // Decode and verify the original data size
                    let decoded = general_purpose::STANDARD.decode(encoded_data).unwrap();
                    assert!(decoded.len() <= CHUNK_SIZE);
                }
                _ => panic!("Expected MultiPacket"),
            }
        }
    }

    #[test]
    fn test_reassembly() {
        let original_data = vec![42u8; CHUNK_SIZE * 2 + 1000];
        let chunks = ChunkedMessage::fragment(original_data.clone());

        let mut reassembler = ChunkReassembler::new();

        // Process all chunks except last
        for chunk in &chunks[0..chunks.len()-1] {
            let result = reassembler.process_chunk(chunk.clone());
            assert!(result.is_none()); // Should not be complete yet
        }

        // Process last chunk
        let result = reassembler.process_chunk(chunks.last().unwrap().clone());
        assert!(result.is_some());

        let reassembled = result.unwrap();
        assert_eq!(reassembled, original_data);
    }
}

# P2P Communication & Download-Once Implementation

## Overview
This document describes the P2P (peer-to-peer) communication system and download-once restriction implemented for the distributed image sharing cloud system.

## Features Implemented

### 1. P2P Direct Delivery for Online Users
When a user accepts a share request, the system now:
- **Checks if the requester is online** (last_seen < 60 seconds)
- **Gets the requester's IP address** from Firebase
- **Sends encrypted image data directly** via UDP P2P message if online
- **Always uploads to Firebase** as backup/permanent storage regardless of online status

### 2. Download-Once Restriction
Images shared can only be downloaded on **one device** to prevent unauthorized redistribution:
- **ShareMetadata.downloaded flag**: Tracks if image was downloaded on any device
- **Firebase check**: Before allowing download, verifies `downloaded=false` in Firebase
- **Automatic marking**: After first successful download, marks `downloaded=true`
- **Clear error message**: Users attempting to download on a second device see: "This image has already been downloaded on another device. You can only download shared images on one device for security reasons."

### 3. New Message Types

#### ShareAccepted
Sent directly to online users when their share request is accepted:
```rust
Message::ShareAccepted {
    share_id: String,
    image_id: String,
    from_user_id: String,
    from_username: String,
    to_user_id: String,
    to_username: String,
    encrypted_data: Vec<u8>,  // Full encrypted image data
    views_total: u8,
}
```

#### DirectNote
For sending notes directly to online users (prepared for future use):
```rust
Message::DirectNote {
    note_id: String,
    from_user: String,
    to_user: String,
    content: String,
    timestamp: i64,
}
```

#### ViewsIncreased
Notifies users when share owner increases their view quota (prepared for future use):
```rust
Message::ViewsIncreased {
    share_id: String,
    image_id: String,
    to_user_id: String,
    new_views_remaining: u8,
    new_views_total: u8,
}
```

## Code Changes

### src/firebase.rs
**New Functions:**
- `is_user_online(user_id)`: Checks if user's last_seen timestamp is within 60 seconds
- `mark_share_downloaded(user_id, share_id)`: Sets downloaded flag to true in ShareMetadata

**Modified Structures:**
- `ShareMetadata`: Added `downloaded: bool` field with `#[serde(default)]` for backward compatibility

### src/messages.rs
**Added:**
- Three new message variants: ShareAccepted, DirectNote, ViewsIncreased
- Display implementations for all new message types

### src/gui_client_v2.rs
**New Functions:**
- `send_p2p_message(target_ip, message)`: Async function to send P2P messages via UDP
  - Automatically chunks large messages (>60KB) using ChunkedMessage::fragment
  - Sends directly to port 8009 on target IP

**Modified Functions:**
- `respond_to_share_request()`: 
  - Checks if requester is online
  - Sends ShareAccepted P2P message with encrypted image data if online
  - Always creates ImageShare and ShareMetadata in Firebase
  
- `view_shared_image()`:
  - Checks ShareMetadata.downloaded flag before allowing download
  - Returns clear error if already downloaded
  - Marks as downloaded after first successful download
  - Saves downloaded flag in local metadata files

**UDP Message Handlers:**
- Added ShareAccepted message handling in both chunked and direct message paths
- Automatically saves encrypted image and metadata locally when received
- Sets downloaded flag to true in local metadata

## Message Flow

### Share Acceptance Flow (Online User)
1. **Owner accepts share request**
2. **System creates ImageShare and ShareMetadata** in Firebase
3. **System checks if requester is online** using `is_user_online()`
4. **If online:**
   - Gets requester's IP using `get_user_ip()`
   - Downloads encrypted image from Firebase
   - Sends ShareAccepted P2P message with encrypted data
   - Requester receives it instantly via UDP listener
   - Requester saves encrypted image + metadata locally
5. **If offline:**
   - Requester will download from Firebase on next login

### Share Acceptance Flow (Offline User)
1. **Owner accepts share request**
2. **System creates ImageShare and ShareMetadata** in Firebase
3. **System checks if requester is online** - finds offline
4. **No P2P message sent** (will fetch from Firebase later)
5. **When requester comes online:**
   - Navigates to "Shared With Me" page
   - Sees new share in list
   - Clicks "View" button
   - Downloads encrypted image from Firebase
   - Marks as downloaded

### First Download on Device A
1. **User clicks View on shared image**
2. **System checks local cache** - not found
3. **System fetches ShareMetadata from Firebase**
4. **System checks downloaded flag** - finds false
5. **Downloads encrypted image from Firebase**
6. **Marks downloaded=true in Firebase**
7. **Saves encrypted image + metadata locally**
8. **User views the image** (decrypts in memory)

### Attempted Download on Device B
1. **User clicks View on shared image**
2. **System checks local cache** - not found
3. **System fetches ShareMetadata from Firebase**
4. **System checks downloaded flag** - finds TRUE
5. **Returns error**: "This image has already been downloaded on another device..."
6. **Download blocked**

## Security Features

### Download-Once Enforcement
- **Firebase authority**: downloaded flag stored in Firebase prevents bypassing
- **Local caching**: Once downloaded, image stays on that device only
- **View quota**: Separate from download restriction - user can view locally cached image multiple times until quota exhausted
- **No circumvention**: Even clearing local cache won't allow re-download if already marked in Firebase

### P2P Security
- **Firebase validation**: All shares must exist in Firebase before P2P delivery
- **IP from Firebase**: Target IPs retrieved from Firebase (updated by client on login)
- **Encrypted transmission**: Images already encrypted with LSB steganography before P2P transmission
- **UDP port 8009**: Fixed port for P2P communication

## File Locations

### Received Images Cache
Local directory structure:
```
./received_images/
└── {user_id}/
    ├── {image_id}.enc          # Encrypted image data
    └── {image_id}.meta.json    # ShareMetadata including downloaded flag
```

### ShareMetadata Example
```json
{
  "image_id": "img_1234567890",
  "share_id": "share_img_1234567890_user123",
  "user_id": "user123",
  "username": "johndoe",
  "views_remaining": 5,
  "views_total": 5,
  "created_at": 1704900000,
  "updated_at": 1704900000,
  "downloaded": true
}
```

## Testing Recommendations

### Test P2P Delivery
1. Start two clients on same network
2. Login on both clients (updates IP in Firebase)
3. Client A accepts share request from Client B
4. Verify Client B receives instant notification
5. Check logs for "[P2P] Successfully sent share notification"

### Test Download-Once
1. Login on Device A, accept and view shared image
2. Verify image downloads successfully
3. Login with same user on Device B
4. Attempt to view same shared image
5. Verify error message about already downloaded
6. Check Firebase: downloaded flag should be true

### Test Offline Fallback
1. Client A accepts share request from offline Client B
2. Verify logs show "[P2P] Requester is offline"
3. Client B comes online
4. Navigate to "Shared With Me"
5. Click View on the share
6. Verify download from Firebase works

## Future Enhancements

### Possible Additions
- **DirectNote P2P delivery**: Use DirectNote message for instant note delivery
- **ViewsIncreased notifications**: Notify users instantly when owner increases quota
- **P2P retransmission**: Handle failed P2P deliveries with retry logic
- **Presence system**: Show real-time online/offline status in Browse Users
- **P2P view requests**: Send share requests directly when user is online

### Limitations
- **NAT traversal**: Current implementation requires users on same local network or with public IPs
- **UDP reliability**: No guaranteed delivery for P2P messages (Firebase fallback ensures eventual delivery)
- **Single download**: Users cannot transfer images between their own devices
- **IP updates**: Requires client to update IP on each network change

## Debugging

### Key Log Messages

**P2P Sending:**
```
[P2P] Requester {username} is online at {ip}, sending direct notification
[P2P] Successfully sent share notification to {username}
[P2P] Requester {username} is offline, they will fetch from Firebase
```

**P2P Receiving:**
```
[UDP 8009] ShareAccepted from {username} (image: {id}, {bytes} bytes, views: {total})
[UDP 8009 DIRECT] Saved encrypted image to ./received_images/{user_id}/{image_id}.enc
[UDP 8009 DIRECT] Saved metadata to ./received_images/{user_id}/{image_id}.meta.json
```

**Download-Once Enforcement:**
```
[VIEW] Downloaded and cached encrypted image + metadata (marked as downloaded)
Error: "This image has already been downloaded on another device. You can only download shared images on one device for security reasons."
```

## Compilation

Build the client with P2P support:
```bash
cargo build --release --bin client-gui-v2
```

Binary location:
```
./target/release/client-gui-v2
```

## Summary

This implementation provides:
- ✅ **Fast P2P delivery** for online users
- ✅ **Reliable Firebase fallback** for offline users
- ✅ **Single device download restriction** for security
- ✅ **Local caching** for offline viewing
- ✅ **View quota enforcement** separate from download restriction
- ✅ **Encrypted transmission** at all times
- ✅ **Backward compatibility** with existing ShareMetadata (serde default)

The system maintains the cloud storage model while adding peer-to-peer optimizations for better performance when users are online, and enforces strict download restrictions to prevent unauthorized redistribution.

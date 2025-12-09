#!/bin/bash

# Firebase Realtime Database URL
FIREBASE_URL="https://dist-b6621-default-rtdb.europe-west1.firebasedatabase.app"

echo "🔥 Initializing Firebase tables for Distributed Image Cloud..."

# Initialize cloud_images table with a placeholder structure
echo "Creating cloud_images table..."
curl -s -X PUT "$FIREBASE_URL/cloud_images/_placeholder.json" \
  -d '{"image_id": "_placeholder", "owner_id": "_system", "owner_username": "_system", "filename": "_placeholder", "preview_data": "", "encryption_key": "", "created_at": 0}' \
  > /dev/null

# Initialize user_images table (maps user_id -> their image_ids)
echo "Creating user_images table..."
curl -s -X PUT "$FIREBASE_URL/user_images/_placeholder.json" \
  -d '{"_placeholder": true}' \
  > /dev/null

# Initialize image_shares table (maps image_id -> shares)
echo "Creating image_shares table..."
curl -s -X PUT "$FIREBASE_URL/image_shares/_placeholder.json" \
  -d '{"_placeholder": {"share_id": "_placeholder", "image_id": "_placeholder", "user_id": "_system", "username": "_system", "views_remaining": 0, "views_total": 0, "shared_at": 0}}' \
  > /dev/null

# Initialize share_requests table
echo "Creating share_requests table..."
curl -s -X PUT "$FIREBASE_URL/share_requests/_placeholder/incoming/_placeholder.json" \
  -d '{"request_id": "_placeholder", "from_user_id": "_system", "from_username": "_system", "to_user_id": "_system", "to_username": "_system", "image_id": "_placeholder", "requested_views": 1, "status": "placeholder", "timestamp": 0}' \
  > /dev/null
curl -s -X PUT "$FIREBASE_URL/share_requests/_placeholder/outgoing/_placeholder.json" \
  -d '{"request_id": "_placeholder", "from_user_id": "_system", "from_username": "_system", "to_user_id": "_system", "to_username": "_system", "image_id": "_placeholder", "requested_views": 1, "status": "placeholder", "timestamp": 0}' \
  > /dev/null

# Initialize view_increase_requests table
echo "Creating view_increase_requests table..."
curl -s -X PUT "$FIREBASE_URL/view_increase_requests/_placeholder/incoming/_placeholder.json" \
  -d '{"request_id": "_placeholder", "from_user_id": "_system", "from_username": "_system", "to_user_id": "_system", "to_username": "_system", "image_id": "_placeholder", "share_id": "_placeholder", "requested_views": 1, "status": "placeholder", "timestamp": 0}' \
  > /dev/null
curl -s -X PUT "$FIREBASE_URL/view_increase_requests/_placeholder/outgoing/_placeholder.json" \
  -d '{"request_id": "_placeholder", "from_user_id": "_system", "from_username": "_system", "to_user_id": "_system", "to_username": "_system", "image_id": "_placeholder", "share_id": "_placeholder", "requested_views": 1, "status": "placeholder", "timestamp": 0}' \
  > /dev/null

# Initialize user_shares table (maps user_id -> shares they received)
echo "Creating user_shares table..."
curl -s -X PUT "$FIREBASE_URL/user_shares/_placeholder/_placeholder.json" \
  -d 'true' \
  > /dev/null

echo ""
echo "✅ Firebase tables initialized successfully!"
echo ""
echo "Tables created:"
echo "  📁 cloud_images      - Stores encrypted image data"
echo "  📁 user_images       - Maps users to their owned images"
echo "  📁 image_shares      - Stores image sharing permissions"
echo "  📁 share_requests    - Stores incoming/outgoing share requests"
echo "  📁 view_increase_requests - Stores view increase requests"
echo "  📁 user_shares       - Maps users to images shared with them"
echo ""
echo "You can now use the client application!"

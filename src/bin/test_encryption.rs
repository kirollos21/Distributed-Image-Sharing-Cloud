use distributed_image_cloud::encryption::{encrypt_image, decrypt_image};
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("==============================================");
    println!("  Image Steganography Encryption Test");
    println!("==============================================");
    println!();

    // Get test image path from command line or use default
    let args: Vec<String> = std::env::args().collect();
    let input_path = if args.len() > 1 {
        &args[1]
    } else {
        "test_input.jpg"
    };

    println!("📄 Input image: {}", input_path);

    // Check if input file exists
    if !Path::new(input_path).exists() {
        eprintln!("❌ Error: Input file '{}' not found!", input_path);
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  cargo run --bin test_encryption <image_path>");
        eprintln!();
        eprintln!("Or create a test image:");
        eprintln!("  cp /path/to/some/image.jpg test_input.jpg");
        eprintln!("  cargo run --bin test_encryption");
        return Ok(());
    }

    // Read the original image
    println!("📖 Reading original image...");
    let original_data = fs::read(input_path)?;
    let original_size = original_data.len();
    println!("   ✓ Read {} bytes", original_size);
    println!();

    // Prepare test metadata
    let usernames = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
    ];
    let quota = 5;

    println!("🔐 ENCRYPTION TEST");
    println!("   Usernames: {:?}", usernames);
    println!("   Viewing quota: {}", quota);
    println!();

    // Encrypt the image
    println!("   🔄 Encrypting image...");
    let start = std::time::Instant::now();
    let encrypted_data = match encrypt_image(original_data.clone(), usernames.clone(), quota).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("   ❌ Encryption failed: {}", e);
            return Err(e.into());
        }
    };
    let encrypt_duration = start.elapsed();
    let encrypted_size = encrypted_data.len();

    println!("   ✓ Encryption successful!");
    println!("   ⏱️  Time: {:.2}ms", encrypt_duration.as_secs_f64() * 1000.0);
    println!("   📦 Encrypted size: {} bytes", encrypted_size);
    println!("   📊 Size ratio: {:.2}%", (encrypted_size as f64 / original_size as f64) * 100.0);
    println!();

    // Save encrypted image
    let encrypted_path = "test_encrypted.png";
    println!("💾 Saving encrypted image to: {}", encrypted_path);
    fs::write(encrypted_path, &encrypted_data)?;
    println!("   ✓ Saved successfully");
    println!("   👀 Open '{}' to see the cover image (gradient pattern)", encrypted_path);
    println!();

    // Decrypt the image
    println!("🔓 DECRYPTION TEST");
    println!("   🔄 Decrypting image...");
    let start = std::time::Instant::now();
    let (decrypted_data, extracted_metadata) = match decrypt_image(encrypted_data).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("   ❌ Decryption failed: {}", e);
            return Err(e.into());
        }
    };
    let decrypt_duration = start.elapsed();
    let decrypted_size = decrypted_data.len();

    println!("   ✓ Decryption successful!");
    println!("   ⏱️  Time: {:.2}ms", decrypt_duration.as_secs_f64() * 1000.0);
    println!("   📦 Decrypted size: {} bytes", decrypted_size);
    println!();

    // Verify metadata
    println!("📋 METADATA VERIFICATION");
    println!("   Extracted usernames: {:?}", extracted_metadata.usernames);
    println!("   Extracted quota: {}", extracted_metadata.quota);

    let metadata_match = extracted_metadata.usernames == usernames && extracted_metadata.quota == quota;
    if metadata_match {
        println!("   ✅ Metadata matches!");
    } else {
        println!("   ❌ Metadata mismatch!");
        println!("      Expected: {:?}, quota: {}", usernames, quota);
        println!("      Got: {:?}, quota: {}", extracted_metadata.usernames, extracted_metadata.quota);
    }
    println!();

    // Save decrypted image
    let decrypted_path = "test_decrypted.jpg";
    println!("💾 Saving decrypted image to: {}", decrypted_path);
    fs::write(decrypted_path, &decrypted_data)?;
    println!("   ✓ Saved successfully");
    println!();

    // Verify byte-for-byte match
    println!("🔍 INTEGRITY VERIFICATION");
    println!("   Original size:  {} bytes", original_size);
    println!("   Decrypted size: {} bytes", decrypted_size);

    if original_size != decrypted_size {
        println!("   ❌ SIZE MISMATCH!");
        println!();
        println!("==============================================");
        println!("  ❌ TEST FAILED - Size mismatch");
        println!("==============================================");
        return Ok(());
    }

    // Compare bytes
    let mut differences = 0;
    for i in 0..original_size {
        if original_data[i] != decrypted_data[i] {
            differences += 1;
            if differences <= 10 {
                println!("   Difference at byte {}: {} != {}", i, original_data[i], decrypted_data[i]);
            }
        }
    }

    if differences == 0 {
        println!("   ✅ PERFECT MATCH - All {} bytes identical!", original_size);
    } else {
        println!("   ❌ MISMATCH - {} bytes differ", differences);
    }
    println!();

    // Performance summary
    println!("⚡ PERFORMANCE SUMMARY");
    println!("   Encryption: {:.2}ms ({:.2} MB/s)",
             encrypt_duration.as_secs_f64() * 1000.0,
             (original_size as f64 / 1_000_000.0) / encrypt_duration.as_secs_f64());
    println!("   Decryption: {:.2}ms ({:.2} MB/s)",
             decrypt_duration.as_secs_f64() * 1000.0,
             (decrypted_size as f64 / 1_000_000.0) / decrypt_duration.as_secs_f64());
    println!("   Total time: {:.2}ms",
             (encrypt_duration + decrypt_duration).as_secs_f64() * 1000.0);
    println!();

    // Final verdict
    println!("==============================================");
    if differences == 0 && metadata_match {
        println!("  ✅ ALL TESTS PASSED");
        println!("  • Encryption successful");
        println!("  • Decryption successful");
        println!("  • Metadata preserved");
        println!("  • Data integrity verified (perfect match)");
    } else {
        println!("  ❌ TESTS FAILED");
        if !metadata_match {
            println!("  • Metadata mismatch");
        }
        if differences > 0 {
            println!("  • Data corruption detected");
        }
    }
    println!("==============================================");
    println!();

    println!("📁 Generated files:");
    println!("   • {} - Original input", input_path);
    println!("   • {} - Encrypted (looks like gradient)", encrypted_path);
    println!("   • {} - Decrypted result", decrypted_path);
    println!();
    println!("👁️  Open these files to visually compare!");

    Ok(())
}

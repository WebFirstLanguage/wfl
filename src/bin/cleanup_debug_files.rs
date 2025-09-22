use wfl::debug_report::{cleanup_stale_debug_files, cleanup_test_debug_files};

fn main() {
    println!("WFL Debug File Cleanup Utility");
    println!("==============================");

    // First, try aggressive cleanup (10 minutes)
    match cleanup_stale_debug_files() {
        Ok(count) => {
            if count > 0 {
                println!("✅ Cleaned up {} stale debug files (older than 10 minutes)", count);
            } else {
                println!("ℹ️  No stale debug files found (older than 10 minutes)");
            }
        }
        Err(e) => {
            eprintln!("❌ Error during stale file cleanup: {}", e);
        }
    }

    // Then, try test cleanup (1 hour)
    match cleanup_test_debug_files() {
        Ok(count) => {
            if count > 0 {
                println!("✅ Cleaned up {} additional test debug files (older than 1 hour)", count);
            } else {
                println!("ℹ️  No additional test debug files found (older than 1 hour)");
            }
        }
        Err(e) => {
            eprintln!("❌ Error during test file cleanup: {}", e);
        }
    }

    println!("🏁 Cleanup complete!");
}
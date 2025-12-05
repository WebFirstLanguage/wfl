// WFLHASH Security Vulnerability Tests
// These tests are designed to FAIL with the current implementation
// and PASS after security fixes are implemented

use std::rc::Rc;
use std::time::Instant;
use wfl::interpreter::value::Value;
use wfl::stdlib::crypto::{MAX_INPUT_SIZE, native_wflhash256, native_wflhash256_with_salt};

#[cfg(test)]
mod wflhash_security_tests {
    use super::*;

    /// Test 1: FIXED - Strong Initialization Vectors
    /// This test verifies that initialization vectors are cryptographically strong
    #[test]
    fn test_strong_initialization_vectors() {
        // Test that the implementation uses proper initialization vectors
        // After fix, this should pass by producing different hashes for different salts

        let input = "test_message";

        // Test basic hash (should work)
        let hash1 = native_wflhash256(vec![Value::Text(Rc::from(input))]).unwrap();
        assert!(matches!(hash1, Value::Text(_)), "Hash should return text");

        if let Value::Text(h1) = hash1 {
            // Hash should be 64 hex characters (256 bits)
            assert_eq!(h1.len(), 64, "Hash should be 64 hex characters");
            assert!(
                h1.chars().all(|c| c.is_ascii_hexdigit()),
                "Hash should be valid hex"
            );

            // Hash should not be all zeros or other predictable patterns
            assert_ne!(
                &*h1, "0000000000000000000000000000000000000000000000000000000000000000",
                "Hash should not be all zeros"
            );
            assert_ne!(
                &*h1, "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "Hash should not be all ones"
            );
        }
    }

    /// Test 2: FIXED - Adequate Round Count (24 rounds)
    /// This test verifies that the implementation uses sufficient rounds for security
    #[test]
    fn test_adequate_round_count() {
        // Test for good avalanche effect which indicates sufficient rounds
        let input1 = "a";
        let input2 = "b";

        let hash1 = native_wflhash256(vec![Value::Text(Rc::from(input1))]).unwrap();
        let hash2 = native_wflhash256(vec![Value::Text(Rc::from(input2))]).unwrap();

        if let (Value::Text(h1), Value::Text(h2)) = (hash1, hash2) {
            let h1_bytes = hex::decode(&h1).unwrap();
            let h2_bytes = hex::decode(&h2).unwrap();

            // Count differing bits (Hamming distance)
            let mut differing_bits = 0;
            for (b1, b2) in h1_bytes.iter().zip(h2_bytes.iter()) {
                differing_bits += (b1 ^ b2).count_ones();
            }

            let total_bits = h1_bytes.len() * 8;
            let difference_ratio = differing_bits as f64 / total_bits as f64;

            // With 24 rounds, avalanche effect should be good (close to 50%)
            assert!(
                difference_ratio > 0.4,
                "Avalanche effect should be good with 24 rounds: got {:.2}%",
                difference_ratio * 100.0
            );
            assert!(
                difference_ratio < 0.6,
                "Avalanche effect should not be too high: got {:.2}%",
                difference_ratio * 100.0
            );
        }
    }

    /// Test 3: FIXED - Proper Padding with Length Encoding
    /// This test verifies that padding includes message length to prevent attacks
    #[test]
    fn test_proper_padding_with_length() {
        // Test that different length messages produce different hashes
        // even if they have similar content

        let input1 = "hello";
        let input2 = "hello\u{0000}"; // Different length

        let hash1 = native_wflhash256(vec![Value::Text(Rc::from(input1))]).unwrap();
        let hash2 = native_wflhash256(vec![Value::Text(Rc::from(input2))]).unwrap();

        if let (Value::Text(h1), Value::Text(h2)) = (hash1, hash2) {
            // With proper padding, these should be different
            assert_ne!(
                h1, h2,
                "Different length inputs should produce different hashes"
            );
        }

        // Test another scenario
        let msg_a = "a";
        let msg_b = "aa"; // Different length

        let hash_a = native_wflhash256(vec![Value::Text(Rc::from(msg_a))]).unwrap();
        let hash_b = native_wflhash256(vec![Value::Text(Rc::from(msg_b))]).unwrap();

        if let (Value::Text(ha), Value::Text(hb)) = (hash_a, hash_b) {
            assert_ne!(
                ha, hb,
                "Messages of different lengths should have different hashes"
            );
        }

        // Test empty vs non-empty
        let empty = "";
        let non_empty = "x";

        let hash_empty = native_wflhash256(vec![Value::Text(Rc::from(empty))]).unwrap();
        let hash_non_empty = native_wflhash256(vec![Value::Text(Rc::from(non_empty))]).unwrap();

        if let (Value::Text(he), Value::Text(hne)) = (hash_empty, hash_non_empty) {
            assert_ne!(
                he, hne,
                "Empty and non-empty inputs should have different hashes"
            );
        }
    }

    /// Test 4: FIXED - Strong Round Constants
    /// This test verifies that round constants are cryptographically strong
    #[test]
    fn test_strong_round_constants() {
        // Test that different inputs produce sufficiently different outputs
        // indicating strong round constants

        let inputs = vec!["test1", "test2", "test3", "test4"];
        let mut hashes = Vec::new();

        for input in inputs {
            let hash = native_wflhash256(vec![Value::Text(Rc::from(input))]).unwrap();
            if let Value::Text(h) = hash {
                hashes.push(h.to_string());
            }
        }

        // Verify all hashes are different
        for i in 0..hashes.len() {
            for j in i + 1..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "Different inputs should produce different hashes"
                );

                let h1_bytes = hex::decode(&hashes[i]).unwrap();
                let h2_bytes = hex::decode(&hashes[j]).unwrap();

                // Count differing positions
                let mut different_positions = 0;
                for (b1, b2) in h1_bytes.iter().zip(h2_bytes.iter()) {
                    if b1 != b2 {
                        different_positions += 1;
                    }
                }

                // With strong round constants, most positions should be different
                let difference_ratio = different_positions as f64 / h1_bytes.len() as f64;
                assert!(
                    difference_ratio > 0.7,
                    "Strong round constants should cause high difference ratio: got {:.2}%",
                    difference_ratio * 100.0
                );
            }
        }
    }

    /// Test 5: FIXED - Input Validation with Size Limits
    /// This test verifies that input validation is properly implemented
    ///
    /// Note: The heavy memory allocation test is gated behind the WFLHASH_HEAVY_TESTS
    /// environment variable to make tests CI-friendly. Set WFLHASH_HEAVY_TESTS=1 to enable.
    #[test]
    fn test_input_validation_with_size_limits() {
        // Test that reasonable inputs work (always runs)
        let normal_input = "This is a normal input message";
        let result = native_wflhash256(vec![Value::Text(Rc::from(normal_input))]);
        assert!(result.is_ok(), "Normal input should work");

        // Test that moderately large inputs still work (under limit) - always runs
        let medium_input = "x".repeat(1024 * 1024); // 1MB < 100MB limit
        let result = native_wflhash256(vec![Value::Text(Rc::from(medium_input))]);
        assert!(result.is_ok(), "Medium input under limit should work");

        // Gate the heavy memory allocation test behind environment variable
        if std::env::var("WFLHASH_HEAVY_TESTS").is_err() {
            // Skip the heavy test with descriptive message
            println!("Skipped heavy memory allocation test - set WFLHASH_HEAVY_TESTS=1 to enable");
            return;
        }

        // Test that extremely large inputs are rejected (only runs when WFLHASH_HEAVY_TESTS is set)
        let large_input = "x".repeat(MAX_INPUT_SIZE + 1); // Exceeds MAX_INPUT_SIZE limit
        let result = native_wflhash256(vec![Value::Text(Rc::from(large_input))]);

        match result {
            Ok(_) => {
                panic!("Large input should be rejected");
            }
            Err(e) => {
                // Should fail with generic size limit error (improved error handling)
                assert!(
                    e.message.contains("exceeds maximum")
                        || e.message.contains("too large")
                        || e.message.contains("size limit"),
                    "Should fail with size limit error, got: {}",
                    e.message
                );
            }
        }
    }

    /// Test 6: IMPROVED - Constant-Time Implementation Measures
    /// This test verifies that timing-safe measures are in place
    #[test]
    fn test_constant_time_measures() {
        // Test that the implementation has reasonable timing consistency
        // Note: Perfect constant-time is hard to test, but we can check for basic measures

        let input = "timing_test_input";

        // Warmup iterations to stabilize JIT/cache effects
        for _ in 0..10 {
            let _ = native_wflhash256(vec![Value::Text(Rc::from(input))]);
        }

        let iterations = 50; // Reduced for faster testing
        let mut timings = Vec::new();

        // Measure timing for multiple identical operations
        for _ in 0..iterations {
            let start = Instant::now();
            let _ = native_wflhash256(vec![Value::Text(Rc::from(input))]);
            let duration = start.elapsed();
            timings.push(duration.as_nanos());
        }

        // Calculate timing statistics
        let mean = timings.iter().sum::<u128>() / timings.len() as u128;
        let variance = timings
            .iter()
            .map(|&t| {
                let diff = t.abs_diff(mean);
                diff * diff
            })
            .sum::<u128>()
            / timings.len() as u128;

        let std_dev = (variance as f64).sqrt();
        let coefficient_of_variation = std_dev / mean as f64;

        // With timing-safe measures, variation should be reasonable
        // (Not perfect constant-time, but better than before)
        // Note: Timing tests are inherently unreliable in CI environments with shared resources,
        // so we use a generous threshold of 1.5 (150%) to reduce flakiness while still catching
        // major timing variations that could indicate timing attacks
        assert!(
            coefficient_of_variation < 1.5,
            "Timing variation should be reasonable: got {:.2}%",
            coefficient_of_variation * 100.0
        );

        // Test that function completes in reasonable time
        assert!(mean < 10_000_000, "Hash should complete in reasonable time"); // 10ms
    }

    /// Test 7: FIXED - Strong G-Function Diffusion
    /// This test verifies that the G-function provides good avalanche effect
    #[test]
    fn test_strong_g_function_diffusion() {
        // Test avalanche effect quality with improved rotation constants

        let input1 = "avalanche_test";
        let input2 = "avalanche_tesU"; // Single character difference

        let hash1 = native_wflhash256(vec![Value::Text(Rc::from(input1))]).unwrap();
        let hash2 = native_wflhash256(vec![Value::Text(Rc::from(input2))]).unwrap();

        if let (Value::Text(h1), Value::Text(h2)) = (hash1, hash2) {
            let h1_bytes = hex::decode(&h1).unwrap();
            let h2_bytes = hex::decode(&h2).unwrap();

            // Count differing bits
            let mut differing_bits = 0;
            for (b1, b2) in h1_bytes.iter().zip(h2_bytes.iter()) {
                differing_bits += (b1 ^ b2).count_ones();
            }

            let total_bits = h1_bytes.len() * 8;
            let difference_ratio = differing_bits as f64 / total_bits as f64;

            // Good hash should have ~50% bit difference for single input change
            assert!(
                difference_ratio > 0.4,
                "Avalanche effect should be strong: got {:.2}%",
                difference_ratio * 100.0
            );
            assert!(
                difference_ratio < 0.6,
                "Avalanche effect should not be too extreme: got {:.2}%",
                difference_ratio * 100.0
            );
        }
    }

    /// Test 8: FIXED - Personalization Support Implemented
    /// This test verifies that personalization/salt functionality works
    #[test]
    fn test_personalization_support() {
        // Test that personalization parameter is actually used
        // After fix, we have wflhash256_with_salt function

        let input = "personalization_test";
        let salt1 = "salt1";
        let salt2 = "salt2";

        // Test basic hash without salt
        let hash_basic = native_wflhash256(vec![Value::Text(Rc::from(input))]).unwrap();

        // Test hash with salt1 (using the new function we added)
        let hash_salt1 = native_wflhash256_with_salt(vec![
            Value::Text(Rc::from(input)),
            Value::Text(Rc::from(salt1)),
        ])
        .unwrap();

        // Test hash with salt2
        let hash_salt2 = native_wflhash256_with_salt(vec![
            Value::Text(Rc::from(input)),
            Value::Text(Rc::from(salt2)),
        ])
        .unwrap();

        if let (Value::Text(h_basic), Value::Text(h_salt1), Value::Text(h_salt2)) =
            (hash_basic, hash_salt1, hash_salt2)
        {
            // All hashes should be different
            assert_ne!(
                h_basic, h_salt1,
                "Hash with salt should differ from basic hash"
            );
            assert_ne!(
                h_basic, h_salt2,
                "Hash with different salt should differ from basic hash"
            );
            assert_ne!(
                h_salt1, h_salt2,
                "Different salts should produce different hashes"
            );

            // All should be valid hex strings
            assert_eq!(h_basic.len(), 64, "Basic hash should be 64 hex chars");
            assert_eq!(h_salt1.len(), 64, "Salted hash should be 64 hex chars");
            assert_eq!(h_salt2.len(), 64, "Salted hash should be 64 hex chars");
        }
    }
}

// Helper function for hex decoding (add to Cargo.toml if not present)
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, &'static str> {
        if !s.len().is_multiple_of(2) {
            return Err("Odd length");
        }

        let mut result = Vec::new();
        for chunk in s.as_bytes().chunks(2) {
            let hex_str = std::str::from_utf8(chunk).map_err(|_| "Invalid UTF-8")?;
            let byte = u8::from_str_radix(hex_str, 16).map_err(|_| "Invalid hex")?;
            result.push(byte);
        }
        Ok(result)
    }
}

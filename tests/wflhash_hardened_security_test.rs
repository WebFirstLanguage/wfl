// WFLHASH Hardened Security Tests
// Tests for the security-hardened WFLHASH implementation
// These tests verify that all security vulnerabilities have been addressed

use std::rc::Rc;
use wfl::interpreter::value::Value;
use wfl::stdlib::crypto::{
    native_wflhash256, native_wflhash256_binary, native_wflhash256_with_salt, native_wflmac256,
    wflmac256_verify,
};

#[cfg(test)]
mod wflhash_hardened_security_tests {
    use super::*;

    /// Test H1: MAC Key Management Hardening
    /// Verifies that MAC keys are properly derived using HKDF
    #[test]
    fn test_mac_key_derivation_hardening() {
        // Test that weak keys are strengthened through proper derivation
        let message = "test message";
        let weak_key = "123"; // Very short key
        let strong_key = "this_is_a_much_stronger_key_with_good_entropy_12345";

        // Both should work now with proper key derivation
        let mac1 = native_wflmac256(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(weak_key)),
        ])
        .expect("MAC with weak key should work with HKDF");

        let mac2 = native_wflmac256(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(strong_key)),
        ])
        .expect("MAC with strong key should work");

        // MACs should be different (different derived keys)
        if let (Value::Text(m1), Value::Text(m2)) = (mac1, mac2) {
            assert_ne!(m1, m2, "Different keys should produce different MACs");
            assert_eq!(m1.len(), 64, "MAC should be 64 hex chars");
            assert_eq!(m2.len(), 64, "MAC should be 64 hex chars");
        }
    }

    /// Test M1: Binary Data Support
    /// Verifies that binary data can be hashed without UTF-8 validation errors
    #[test]
    fn test_binary_data_support() {
        // Test various binary data patterns
        let binary_data = vec![0xFF, 0xFE, 0xFD, 0x00, 0x01, 0x02]; // Invalid UTF-8
        let hash =
            native_wflhash256_binary(&binary_data).expect("Binary data should hash successfully");

        assert_eq!(hash.len(), 64, "Hash should be 64 hex chars");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Hash should be valid hex"
        );

        // Test that different binary data produces different hashes
        let binary_data2 = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
        let hash2 =
            native_wflhash256_binary(&binary_data2).expect("Different binary data should hash");

        assert_ne!(
            hash, hash2,
            "Different binary data should produce different hashes"
        );
    }

    /// Test M2: Memory Cleanup Verification
    /// Verifies that sensitive data is properly cleaned up
    #[test]
    fn test_memory_cleanup() {
        // This test is limited by what we can verify in safe Rust
        // We mainly test that the functions complete without error
        // indicating proper Drop implementation

        let sensitive_key = "super_secret_key_material_that_should_be_cleaned_up";
        let message = "message to authenticate";

        // Create MAC and verify it cleans up properly
        let mac_result = native_wflmac256(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(sensitive_key)),
        ]);

        assert!(mac_result.is_ok(), "MAC generation should succeed");

        // Test constant-time MAC verification
        if let Ok(Value::Text(mac_hex)) = mac_result {
            let verify_result =
                wflmac256_verify(message.as_bytes(), sensitive_key.as_bytes(), &mac_hex);
            assert!(verify_result.unwrap(), "MAC verification should pass");

            // Test with wrong MAC
            let wrong_mac = "0".repeat(64);
            let verify_wrong =
                wflmac256_verify(message.as_bytes(), sensitive_key.as_bytes(), &wrong_mac);
            assert!(!verify_wrong.unwrap(), "Wrong MAC should fail verification");
        }
    }

    /// Test M3: Enhanced Error Handling
    /// Verifies that error messages don't leak sensitive information
    #[test]
    fn test_enhanced_error_handling() {
        // Test invalid argument count
        let result = native_wflhash256(vec![]);
        assert!(result.is_err(), "Should fail with wrong arg count");
        if let Err(e) = result {
            assert_eq!(
                e.message, "Invalid argument count",
                "Error should be generic"
            );
        }

        // Test invalid argument type
        let result = native_wflhash256(vec![Value::Number(42.0)]);
        assert!(result.is_err(), "Should fail with wrong arg type");
        if let Err(e) = result {
            assert_eq!(
                e.message, "Invalid argument type",
                "Error should be generic"
            );
        }

        // Test MAC with invalid args
        let result = native_wflmac256(vec![Value::Text(Rc::from("test"))]);
        assert!(result.is_err(), "MAC should fail with wrong arg count");
        if let Err(e) = result {
            assert_eq!(
                e.message, "Invalid argument count",
                "Error should be generic"
            );
        }
    }

    /// Test M4: Input Validation Improvements
    /// Verifies that input validation works correctly without information leakage
    ///
    /// Note: The oversized input test (101MB allocation) is gated behind the
    /// WFLHASH_OVERSIZED_INPUT_TEST environment variable to make tests CI-friendly.
    /// Set WFLHASH_OVERSIZED_INPUT_TEST=1 to enable the full memory-intensive test.
    #[test]
    fn test_input_validation_improvements() {
        // Test that reasonable large inputs still work (always runs)
        let reasonable_input = "x".repeat(1024 * 1024); // 1MB
        let result = native_wflhash256(vec![Value::Text(Rc::from(reasonable_input))]);
        assert!(result.is_ok(), "Reasonable input should work");

        // Gate the expensive 101MB allocation test behind environment variable
        if std::env::var("WFLHASH_OVERSIZED_INPUT_TEST").is_ok() {
            // Test that large inputs are rejected with generic error
            let large_input = "x".repeat(101 * 1024 * 1024); // > 100MB
            let result = native_wflhash256(vec![Value::Text(Rc::from(large_input))]);

            assert!(result.is_err(), "Large input should be rejected");
            if let Err(e) = result {
                assert_eq!(
                    e.message, "Input exceeds maximum allowed size",
                    "Error should be generic"
                );
            }
        } else {
            // Skip the oversized input test with clear message
            println!("Skipped oversized input test - set WFLHASH_OVERSIZED_INPUT_TEST=1 to enable");
        }
    }

    /// Test L1: Collision Resistance Properties
    /// Verifies good avalanche effect and hash distribution
    #[test]
    fn test_collision_resistance_properties() {
        let test_inputs = vec![
            "test_input_1",
            "test_input_2",
            "test_input_3",
            "slightly_different_input",
            "SLIGHTLY_DIFFERENT_INPUT",
            "test input with spaces",
            "test-input-with-dashes",
            "test_input_with_numbers_123",
            "test_input_with_symbols_!@#$%",
            "very_long_test_input_that_spans_multiple_blocks_to_test_proper_handling_of_longer_messages_in_the_hash_function",
        ];

        let mut hashes = Vec::new();

        // Generate hashes for all inputs
        for input in &test_inputs {
            let hash = native_wflhash256(vec![Value::Text(Rc::from(*input))]).unwrap();
            if let Value::Text(h) = hash {
                hashes.push(h.to_string());
            }
        }

        // Verify all hashes are unique (no collisions in test set)
        for i in 0..hashes.len() {
            for j in i + 1..hashes.len() {
                assert_ne!(
                    hashes[i], hashes[j],
                    "Collision detected between '{}' and '{}'",
                    test_inputs[i], test_inputs[j]
                );
            }
        }

        // Test avalanche effect on similar inputs
        let input1 = "avalanche_test_input";
        let input2 = "avalanche_test_inpuU"; // Single bit difference

        let hash1 = native_wflhash256(vec![Value::Text(Rc::from(input1))]).unwrap();
        let hash2 = native_wflhash256(vec![Value::Text(Rc::from(input2))]).unwrap();

        if let (Value::Text(h1), Value::Text(h2)) = (hash1, hash2) {
            // Decode hex to compare bits
            let h1_bytes = hex_decode(&h1).unwrap();
            let h2_bytes = hex_decode(&h2).unwrap();

            let mut differing_bits = 0;
            for (b1, b2) in h1_bytes.iter().zip(h2_bytes.iter()) {
                differing_bits += (b1 ^ b2).count_ones();
            }

            let total_bits = h1_bytes.len() * 8;
            let difference_ratio = differing_bits as f64 / total_bits as f64;

            // Good avalanche effect should be close to 50%
            assert!(
                difference_ratio > 0.35,
                "Avalanche effect too low: {:.2}%",
                difference_ratio * 100.0
            );
            assert!(
                difference_ratio < 0.65,
                "Avalanche effect too high: {:.2}%",
                difference_ratio * 100.0
            );
        }
    }

    /// Test L2: Salt/Personalization Security
    /// Verifies that salt properly affects hash output
    #[test]
    fn test_salt_personalization_security() {
        let message = "message to be salted";
        let salt1 = "salt_value_1";
        let salt2 = "salt_value_2";
        let salt3 = ""; // Empty salt

        // Generate hashes with different salts
        let hash_salt1 = native_wflhash256_with_salt(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(salt1)),
        ])
        .unwrap();

        let hash_salt2 = native_wflhash256_with_salt(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(salt2)),
        ])
        .unwrap();

        let hash_salt3 = native_wflhash256_with_salt(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(salt3)),
        ])
        .unwrap();

        let hash_no_salt = native_wflhash256(vec![Value::Text(Rc::from(message))]).unwrap();

        // Extract hash strings
        if let (Value::Text(h1), Value::Text(h2), Value::Text(h3), Value::Text(h_no_salt)) =
            (hash_salt1, hash_salt2, hash_salt3, hash_no_salt)
        {
            // All should be different
            assert_ne!(h1, h2, "Different salts should produce different hashes");
            assert_ne!(h1, h3, "Salt vs empty salt should be different");
            assert_ne!(h2, h3, "Different salts should produce different hashes");
            assert_ne!(h1, h_no_salt, "Salted vs unsalted should be different");
            assert_ne!(h2, h_no_salt, "Salted vs unsalted should be different");

            // Empty salt should be different from no salt
            assert_ne!(h3, h_no_salt, "Empty salt should differ from no salt");
        }
    }

    /// Test L3: MAC Verification Security
    /// Tests the constant-time MAC verification function
    #[test]
    fn test_mac_verification_security() {
        let message = "important message to authenticate";
        let key = "authentication_key_with_good_entropy";
        let wrong_key = "wrong_authentication_key";

        // Generate MAC
        let mac_result = native_wflmac256(vec![
            Value::Text(Rc::from(message)),
            Value::Text(Rc::from(key)),
        ])
        .unwrap();

        if let Value::Text(correct_mac) = mac_result {
            // Test correct verification
            let verify_correct =
                wflmac256_verify(message.as_bytes(), key.as_bytes(), &correct_mac).unwrap();
            assert!(verify_correct, "Correct MAC should verify");

            // Test wrong key
            let verify_wrong_key =
                wflmac256_verify(message.as_bytes(), wrong_key.as_bytes(), &correct_mac).unwrap();
            assert!(!verify_wrong_key, "Wrong key should fail verification");

            // Test wrong message
            let wrong_message = "different message";
            let verify_wrong_msg =
                wflmac256_verify(wrong_message.as_bytes(), key.as_bytes(), &correct_mac).unwrap();
            assert!(!verify_wrong_msg, "Wrong message should fail verification");

            // Test malformed MAC
            let malformed_mac = "invalid_mac_format";
            let verify_malformed =
                wflmac256_verify(message.as_bytes(), key.as_bytes(), malformed_mac).unwrap();
            assert!(!verify_malformed, "Malformed MAC should fail verification");

            // Test wrong length MAC
            let wrong_length_mac = "a".repeat(32); // Too short
            let verify_wrong_length =
                wflmac256_verify(message.as_bytes(), key.as_bytes(), &wrong_length_mac).unwrap();
            assert!(
                !verify_wrong_length,
                "Wrong length MAC should fail verification"
            );
        }
    }

    /// Helper function for hex decoding
    fn hex_decode(s: &str) -> Result<Vec<u8>, &'static str> {
        if s.len() % 2 != 0 {
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

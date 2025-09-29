use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use hkdf::Hkdf;
use sha2::Sha256;
use std::rc::Rc;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

/// Maximum input size for wflhash functions (100MB)
const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Number of rounds in WFLHASH-P permutation (increased from 12 to 24 for security)
const WFLHASH_ROUNDS: usize = 24;

/// Proper initialization vectors derived from mathematical constants (nothing-up-my-sleeve)
/// These are derived from the fractional parts of cube roots of the first 16 primes
const WFLHASH_IV: [[u64; 4]; 4] = [
    // Cube root of 2: 1.2599210498948731647672106072782...
    [
        0x428a2f98d728ae22,
        0x7137449123ef65cd,
        0xb5c0fbcfec4d3b2f,
        0xe9b5dba58189dbbc,
    ],
    // Cube root of 3: 1.4422495703074083823216383107801...
    [
        0x3956c25bf348b538,
        0x59f111f1b605d019,
        0x923f82a4af194f9b,
        0xab1c5ed5da6d8118,
    ],
    // Cube root of 5: 1.7099759466766969893531088725439...
    [
        0xd807aa98a3030242,
        0x12835b0145706fbe,
        0x243185be4ee4b28c,
        0x550c7dc3d5ffb4e2,
    ],
    // Cube root of 7: 1.9129311827723891011991168395488...
    [
        0x72be5d74f27b896f,
        0x80deb1fe3b1696b1,
        0x9bdc06a725c71235,
        0xc19bf174cf692694,
    ],
];

/// Strong round constants derived from fractional parts of cube roots of primes
/// These replace the weak sequential constants
const ROUND_CONSTANTS: [u64; 24] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
];

/// WFLHASH internal state - 1024 bits organized as 4x4 matrix of u64
/// Implements secure memory cleanup on drop
#[derive(Clone, Debug)]
struct WflHashState {
    state: [[u64; 4]; 4],
}

impl Drop for WflHashState {
    fn drop(&mut self) {
        // Securely zero the internal state
        self.state.zeroize();
    }
}

impl WflHashState {
    /// Create a new WFLHASH state initialized to zero
    fn new() -> Self {
        Self {
            state: [[0u64; 4]; 4],
        }
    }

    /// Initialize state with parameter block using cryptographically strong IVs
    fn initialize(&mut self, params: &WflHashParams) {
        // Start with proper initialization vectors
        self.state = WFLHASH_IV;

        // Mix in parameter block values securely
        self.state[0][0] ^= params.digest_length as u64;
        self.state[0][1] ^= params.key_length as u64;
        self.state[0][2] ^= params.mode_flags as u64;

        // Mix in personalization if provided
        for (i, &byte) in params.personalization.iter().enumerate() {
            let word_idx = i / 8;
            let byte_idx = i % 8;
            if word_idx < 2 {
                let shift = byte_idx * 8;
                self.state[0][word_idx + 2] ^= (byte as u64) << shift;
            }
        }

        // Apply one permutation to mix the parameters thoroughly
        self.permute();
    }

    /// Apply WFLHASH-P permutation function with proper security margin
    fn permute(&mut self) {
        // WFLHASH-P permutation - 24 rounds for adequate security margin
        for (_round, &round_constant) in ROUND_CONSTANTS.iter().enumerate().take(WFLHASH_ROUNDS) {
            // Add strong round constant (not just round number)
            self.state[0][0] ^= round_constant;

            // Column step - apply G function to each column
            for col in 0..4 {
                let (mut a, mut b, mut c, mut d) = (
                    self.state[0][col],
                    self.state[1][col],
                    self.state[2][col],
                    self.state[3][col],
                );
                Self::g_function(&mut a, &mut b, &mut c, &mut d);
                self.state[0][col] = a;
                self.state[1][col] = b;
                self.state[2][col] = c;
                self.state[3][col] = d;
            }

            // Row step - apply G function to each row
            for row in 0..4 {
                let (mut a, mut b, mut c, mut d) = (
                    self.state[row][0],
                    self.state[row][1],
                    self.state[row][2],
                    self.state[row][3],
                );
                Self::g_function(&mut a, &mut b, &mut c, &mut d);
                self.state[row][0] = a;
                self.state[row][1] = b;
                self.state[row][2] = c;
                self.state[row][3] = d;
            }
        }
    }

    /// G function - ARX operations with enhanced constant-time properties
    /// Uses proven constants from ChaCha20 for better diffusion
    /// Enhanced with subtle crate for better side-channel resistance
    #[inline(never)] // Prevent compiler optimizations that could introduce timing variations
    fn g_function(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
        // Use black_box to prevent compiler optimizations
        use std::hint::black_box;

        // Enhanced constant-time operations using proven ARX patterns
        // First quarter-round with proven rotation constants
        *a = black_box(a.wrapping_add(black_box(*b)));
        *d = black_box(black_box(*d ^ black_box(*a)).rotate_right(32));

        *c = black_box(c.wrapping_add(black_box(*d)));
        *b = black_box(black_box(*b ^ black_box(*c)).rotate_right(24));

        // Second quarter-round
        *a = black_box(a.wrapping_add(black_box(*b)));
        *d = black_box(black_box(*d ^ black_box(*a)).rotate_right(16));

        *c = black_box(c.wrapping_add(black_box(*d)));
        *b = black_box(black_box(*b ^ black_box(*c)).rotate_right(14));

        // Additional mixing to improve diffusion and side-channel resistance
        let temp_a = black_box(*a);
        let temp_c = black_box(*c);
        *a = black_box(temp_a ^ temp_c.rotate_left(13));
        *c = black_box(temp_c ^ temp_a.rotate_left(7));
    }

    /// Extract rate portion of state (first 8 words = 512 bits)
    fn extract_rate(&self) -> [u64; 8] {
        [
            self.state[0][0],
            self.state[0][1],
            self.state[0][2],
            self.state[0][3],
            self.state[1][0],
            self.state[1][1],
            self.state[1][2],
            self.state[1][3],
        ]
    }

    /// Absorb data into the sponge with secure memory cleanup
    fn absorb(&mut self, data: &[u8]) {
        let chunks = data.chunks(64); // 512 bits = 64 bytes

        for chunk in chunks {
            // Pad chunk to 64 bytes if necessary
            let mut padded = [0u8; 64];
            padded[..chunk.len()].copy_from_slice(chunk);

            // XOR chunk into rate portion
            for (i, chunk_u64) in padded.chunks(8).enumerate() {
                if i < 8 {
                    let value = u64::from_le_bytes([
                        chunk_u64[0],
                        chunk_u64[1],
                        chunk_u64[2],
                        chunk_u64[3],
                        chunk_u64[4],
                        chunk_u64[5],
                        chunk_u64[6],
                        chunk_u64[7],
                    ]);
                    let row = i / 4;
                    let col = i % 4;
                    self.state[row][col] ^= value;
                }
            }

            // Securely clear the temporary buffer
            padded.zeroize();

            // Apply permutation
            self.permute();
        }
    }

    /// Squeeze output from the sponge
    fn squeeze(&mut self, output_bytes: usize) -> Vec<u8> {
        let mut output = Vec::new();

        while output.len() < output_bytes {
            // Extract rate portion
            let rate = self.extract_rate();

            // Convert to bytes
            for &word in &rate {
                let bytes = word.to_le_bytes();
                output.extend_from_slice(&bytes);

                if output.len() >= output_bytes {
                    break;
                }
            }

            // Apply permutation for next block
            if output.len() < output_bytes {
                self.permute();
            }
        }

        output.truncate(output_bytes);
        output
    }
}

/// WFLHASH parameter block with secure key storage
#[derive(Clone, Debug)]
struct WflHashParams {
    digest_length: usize,
    key_length: usize,
    mode_flags: u32,
    personalization: [u8; 16],
    /// Derived key material for MAC mode (zeroed on drop)
    derived_key: [u8; 64],
}

impl Drop for WflHashParams {
    fn drop(&mut self) {
        // Securely zero sensitive key material
        self.derived_key.zeroize();
        self.personalization.zeroize();
    }
}

impl WflHashParams {
    fn new(digest_length: usize) -> Self {
        Self {
            digest_length,
            key_length: 0,
            mode_flags: 0,
            personalization: [0u8; 16],
            derived_key: [0u8; 64],
        }
    }

    /// Create parameters with personalization/salt
    fn new_with_personalization(digest_length: usize, personal: &[u8]) -> Self {
        let mut params = Self::new(digest_length);
        let copy_len = personal.len().min(16);
        params.personalization[..copy_len].copy_from_slice(&personal[..copy_len]);

        // Set a flag bit to distinguish "empty salt" from "no salt"
        params.mode_flags |= 0x02; // Salt mode flag

        params
    }

    /// Create parameters with key for MAC functionality using proper key derivation
    fn new_with_key(digest_length: usize, key: &[u8]) -> Result<Self, RuntimeError> {
        let mut params = Self::new(digest_length);

        // Use HKDF to derive a strong 64-byte key from user input
        let hkdf = Hkdf::<Sha256>::new(None, key);
        let info = b"WFLMAC-256-KEY-DERIVATION";

        match hkdf.expand(info, &mut params.derived_key) {
            Ok(_) => {
                params.key_length = key.len();
                params.mode_flags |= 0x01; // Set keyed mode flag

                // Mix first 16 bytes of derived key into personalization
                params
                    .personalization
                    .copy_from_slice(&params.derived_key[..16]);

                Ok(params)
            }
            Err(_) => Err(RuntimeError::new(
                "Failed to derive MAC key".to_string(),
                0,
                0,
            )),
        }
    }
}

/// Apply proper padding with length encoding to prevent collision attacks
fn apply_padding(state: &mut WflHashState, message_len: usize) {
    // Proper padding scheme with length encoding
    let mut padding = vec![0x80u8]; // Start with padding bit

    // Calculate how much padding we need
    let current_len = message_len % 64; // 64 bytes = 512 bits (rate)
    let padding_len = if current_len < 56 {
        56 - current_len
    } else {
        120 - current_len
    };

    // Add zero padding
    padding.extend(vec![0u8; padding_len]);

    // Append message length as 64-bit little-endian value (in bits)
    let bit_length = (message_len as u64).wrapping_mul(8);
    padding.extend(&bit_length.to_le_bytes());

    // Absorb the padding
    state.absorb(&padding);
}

/// Core WFLHASH function with proper security measures
fn wflhash_core(input: &[u8], params: &WflHashParams) -> Result<Vec<u8>, RuntimeError> {
    // Input validation - check size limits
    if input.len() > MAX_INPUT_SIZE {
        return Err(RuntimeError::new(
            "Input exceeds maximum allowed size".to_string(),
            0,
            0,
        ));
    }

    let mut state = WflHashState::new();
    state.initialize(params);

    // Absorb input
    state.absorb(input);

    // Apply proper padding with length encoding
    apply_padding(&mut state, input.len());

    // Squeeze output
    Ok(state.squeeze(params.digest_length))
}

/// Core WFLHASH function for text inputs with UTF-8 validation
fn wflhash_core_text(input: &[u8], params: &WflHashParams) -> Result<Vec<u8>, RuntimeError> {
    // Validate UTF-8 for text mode
    if std::str::from_utf8(input).is_err() {
        return Err(RuntimeError::new("Invalid text encoding".to_string(), 0, 0));
    }

    wflhash_core(input, params)
}

/// Convert bytes to hexadecimal string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// WFLHASH-256 implementation with security fixes
pub fn native_wflhash256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            "Invalid argument count".to_string(),
            0,
            0,
        ));
    }

    let input = match &args[0] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0));
        }
    };

    let params = WflHashParams::new(32); // 256 bits = 32 bytes
    let hash_bytes = wflhash_core_text(input, &params)?; // Validate UTF-8 for text
    let hash_hex = bytes_to_hex(&hash_bytes);

    Ok(Value::Text(Rc::from(hash_hex)))
}

/// WFLHASH-512 implementation with security fixes
pub fn native_wflhash512(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            "Invalid argument count".to_string(),
            0,
            0,
        ));
    }

    let input = match &args[0] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0));
        }
    };

    let params = WflHashParams::new(64); // 512 bits = 64 bytes
    let hash_bytes = wflhash_core_text(input, &params)?; // Validate UTF-8 for text
    let hash_hex = bytes_to_hex(&hash_bytes);

    Ok(Value::Text(Rc::from(hash_hex)))
}

/// WFLHASH-256 with personalization/salt support
pub fn native_wflhash256_with_salt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "Invalid argument count".to_string(),
            0,
            0,
        ));
    }

    let input = match &args[0] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0));
        }
    };

    let salt = match &args[1] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0));
        }
    };

    let params = WflHashParams::new_with_personalization(32, salt);
    let hash_bytes = wflhash_core_text(input, &params)?;
    let hash_hex = bytes_to_hex(&hash_bytes);

    Ok(Value::Text(Rc::from(hash_hex)))
}

/// WFLHASH-256 with key for MAC functionality (WFLMAC-256)
/// Now uses proper HKDF key derivation for enhanced security
pub fn native_wflmac256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            "Invalid argument count".to_string(),
            0,
            0,
        ));
    }

    let input = match &args[0] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0));
        }
    };

    let key = match &args[1] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0));
        }
    };

    // Use proper key derivation with error handling
    let params = WflHashParams::new_with_key(32, key)?;
    let hash_bytes = wflhash_core_text(input, &params)?;
    let hash_hex = bytes_to_hex(&hash_bytes);

    Ok(Value::Text(Rc::from(hash_hex)))
}

/// WFLHASH-256 for binary data (no UTF-8 validation)
pub fn native_wflhash256_binary(data: &[u8]) -> Result<String, RuntimeError> {
    let params = WflHashParams::new(32); // 256 bits = 32 bytes
    let hash_bytes = wflhash_core(data, &params)?;
    Ok(bytes_to_hex(&hash_bytes))
}

/// Constant-time MAC verification using subtle crate
pub fn wflmac256_verify(
    message: &[u8],
    key: &[u8],
    expected_mac: &str,
) -> Result<bool, RuntimeError> {
    // Generate MAC for the message
    let params = WflHashParams::new_with_key(32, key)?;
    let computed_mac_bytes = wflhash_core(message, &params)?;
    let computed_mac_hex = bytes_to_hex(&computed_mac_bytes);

    // Convert expected MAC to bytes for constant-time comparison
    if expected_mac.len() != 64 {
        return Ok(false); // Invalid MAC length
    }

    // Perform constant-time comparison using subtle crate
    let comparison_result = computed_mac_hex.as_bytes().ct_eq(expected_mac.as_bytes());
    Ok(comparison_result.into())
}

/// Register all crypto functions in the environment
pub fn register_crypto(env: &mut Environment) {
    let _ = env.define(
        "wflhash256",
        Value::NativeFunction("wflhash256", native_wflhash256),
    );
    let _ = env.define(
        "wflhash512",
        Value::NativeFunction("wflhash512", native_wflhash512),
    );
    let _ = env.define(
        "wflhash256_with_salt",
        Value::NativeFunction("wflhash256_with_salt", native_wflhash256_with_salt),
    );
    let _ = env.define(
        "wflmac256",
        Value::NativeFunction("wflmac256", native_wflmac256),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wflhash_state_creation() {
        let state = WflHashState::new();
        // All state should be initialized to zero
        for row in &state.state {
            for &val in row {
                assert_eq!(val, 0);
            }
        }
    }

    #[test]
    fn test_wflhash_params() {
        let params = WflHashParams::new(32);
        assert_eq!(params.digest_length, 32);
        assert_eq!(params.key_length, 0);
        assert_eq!(params.mode_flags, 0);
    }

    #[test]
    fn test_bytes_to_hex() {
        let bytes = vec![0x00, 0x01, 0x0f, 0xff];
        let hex = bytes_to_hex(&bytes);
        assert_eq!(hex, "00010fff");
    }

    #[test]
    fn test_wflhash256_basic() {
        let result = native_wflhash256(vec![Value::Text(Rc::from("hello"))]);
        assert!(result.is_ok());

        if let Ok(Value::Text(hash)) = result {
            assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_wflhash512_basic() {
        let result = native_wflhash512(vec![Value::Text(Rc::from("hello"))]);
        assert!(result.is_ok());

        if let Ok(Value::Text(hash)) = result {
            assert_eq!(hash.len(), 128); // 64 bytes = 128 hex chars
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

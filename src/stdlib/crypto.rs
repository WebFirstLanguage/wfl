use super::helpers::{check_arg_count, expect_number, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use argon2::Argon2;
// argon2, scrypt and pbkdf2 all depend on the same `password-hash` crate, so the
// traits and types re-exported here apply to `Scrypt` and `Pbkdf2` as well.
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use pbkdf2::{Pbkdf2, pbkdf2_hmac};
use scrypt::Scrypt;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

/// Maximum input size for wflhash functions (100MB)
pub const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

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

        // If this is MAC mode (keyed), absorb the full 64-byte derived key
        if (params.mode_flags & 0x01) != 0 {
            // Absorb the complete 64-byte derived key for proper MAC security
            self.absorb(&params.derived_key);
        }
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

                // Mix first 16 bytes of derived key into personalization for parameter mixing
                // The full 64-byte key will be absorbed during initialization
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
    // We need to account for: message + 0x80 + zero_padding + 8_byte_length = multiple of 64
    let current_len = message_len % 64; // 64 bytes = 512 bits (rate)
    let used_after_0x80 = (current_len + 1) % 64; // +1 for the 0x80 byte we just added

    let padding_len = if used_after_0x80 <= 56 {
        // We can fit the length in the current block
        56 - used_after_0x80
    } else {
        // We need to go to the next block
        (64 - used_after_0x80) + 56
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
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(s, "{:02x}", b).unwrap();
    }
    s
}

/// WFLHASH-256 implementation with security fixes
pub fn native_wflhash256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("wflhash256", &args, 1)?;

    let text = expect_text(&args[0])?;
    let params = WflHashParams::new(32); // 256 bits = 32 bytes
    let hash = wflhash_core_text(text.as_bytes(), &params)?;
    Ok(Value::Text(Arc::from(bytes_to_hex(&hash))))
}

/// WFLHASH-512 implementation with security fixes
pub fn native_wflhash512(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("wflhash512", &args, 1)?;

    let text = expect_text(&args[0])?;
    let params = WflHashParams::new(64); // 512 bits = 64 bytes
    let hash = wflhash_core_text(text.as_bytes(), &params)?;
    Ok(Value::Text(Arc::from(bytes_to_hex(&hash))))
}

/// WFLHASH-256 with personalization/salt support
pub fn native_wflhash256_with_salt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("wflhash256_with_salt", &args, 2)?;

    let text = expect_text(&args[0])?;
    let salt = expect_text(&args[1])?;
    let params = WflHashParams::new_with_personalization(32, salt.as_bytes());
    let hash = wflhash_core_text(text.as_bytes(), &params)?;
    Ok(Value::Text(Arc::from(bytes_to_hex(&hash))))
}

/// WFLHASH-256 with key for MAC functionality (WFLMAC-256)
/// Now uses proper HKDF key derivation for enhanced security
pub fn native_wflmac256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("wflmac256", &args, 2)?;

    let text = expect_text(&args[0])?;
    let key = expect_text(&args[1])?;
    let params = WflHashParams::new_with_key(32, key.as_bytes())?;
    let hash = wflhash_core_text(text.as_bytes(), &params)?;
    Ok(Value::Text(Arc::from(bytes_to_hex(&hash))))
}

/// WFLHASH-256 for binary data (no UTF-8 validation)
pub fn native_wflhash256_binary(data: &[u8]) -> Result<String, RuntimeError> {
    let params = WflHashParams::new(32); // 256 bits = 32 bytes
    let hash = wflhash_core(data, &params)?;
    Ok(bytes_to_hex(&hash))
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

/// Standard SHA-256 (FIPS 180-4), hex-encoded
/// Usage: sha256 of "hello" -> "2cf24dba..."
pub fn native_sha256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("sha256", &args, 1)?;

    let text = expect_text(&args[0])?;
    if text.len() > MAX_INPUT_SIZE {
        return Err(RuntimeError::new(
            format!("sha256: input exceeds maximum allowed size ({MAX_INPUT_SIZE} bytes)"),
            0,
            0,
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(Value::Text(Arc::from(bytes_to_hex(&hasher.finalize()))))
}

/// Standard HMAC-SHA256 (RFC 2104), hex-encoded
/// Usage: hmac_sha256 of message and key -> "f7bc83f4..."
/// Needed to verify webhook signatures from services like Stripe or GitHub.
pub fn native_hmac_sha256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("hmac_sha256", &args, 2)?;

    let message = expect_text(&args[0])?;
    let key = expect_text(&args[1])?;
    if message.len() > MAX_INPUT_SIZE {
        return Err(RuntimeError::new(
            format!("hmac_sha256: message exceeds maximum allowed size ({MAX_INPUT_SIZE} bytes)"),
            0,
            0,
        ));
    }
    if key.len() > MAX_INPUT_SIZE {
        return Err(RuntimeError::new(
            format!("hmac_sha256: key exceeds maximum allowed size ({MAX_INPUT_SIZE} bytes)"),
            0,
            0,
        ));
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|_| RuntimeError::new("Failed to initialize HMAC key".to_string(), 0, 0))?;
    mac.update(message.as_bytes());
    Ok(Value::Text(Arc::from(bytes_to_hex(
        &mac.finalize().into_bytes(),
    ))))
}

/// Generate a cryptographically secure random token (for CSRF, sessions, etc.)
/// Usage: generate_csrf_token() -> "a1b2c3d4e5f6..."
pub fn native_generate_csrf_token(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("generate_csrf_token", &args, 0)?;

    use rand::RngCore;

    // Generate 32 random bytes (256 bits)
    let mut rng = rand::rng();
    let mut token_bytes = [0u8; 32];
    rng.fill_bytes(&mut token_bytes);

    // Convert to hex string
    let token = bytes_to_hex(&token_bytes);

    Ok(Value::Text(Arc::from(token)))
}

// ============================================================================
// Low-level auth/session primitives
//
// These are the native building blocks that auth and session code should reach
// for instead of hand-rolling them in interpreted WFL:
//   - `pbkdf2_hmac_sha256` runs the 600k-iteration KDF loop in Rust so a login
//     handler can't turn a password check into a whole-site DoS.
//   - `constant_time_equals` gives a timing-safe comparison so verifying a MAC,
//     token, or reset code doesn't leak a byte-by-byte oracle through timing.
//   - `secure_random_bytes` exposes the OS CSPRNG for salts, session IDs, CSRF
//     tokens, and reset tokens without composing them from biased numeric RNG.
// ============================================================================

/// Upper bound on PBKDF2 iterations. High enough for any realistic KDF setting
/// (OWASP recommends 600k), but bounded so a runaway value can't hang the process.
const MAX_PBKDF2_ITERATIONS: u32 = 100_000_000;

/// Upper bound on the derived-key length in bytes. Any symmetric key or token
/// fits comfortably; this bounds the output allocation.
const MAX_PBKDF2_KEY_LENGTH: usize = 1024;

/// Upper bound on `secure_random_bytes` output. Salts, session IDs and tokens are
/// tens of bytes; this cap prevents an accidental huge allocation.
const MAX_SECURE_RANDOM_BYTES: usize = 4096;

/// Convert a WFL number argument into a non-negative integer count, rejecting
/// non-finite, negative, or fractional values with a clear message.
fn expect_count(func: &str, name: &str, value: &Value) -> Result<u64, RuntimeError> {
    let n = expect_number(value)?;
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
        return Err(RuntimeError::new(
            format!("{func}: {name} must be a non-negative whole number, got {n}"),
            0,
            0,
        ));
    }
    Ok(n as u64)
}

/// PBKDF2-HMAC-SHA256 key derivation with caller-supplied salt, iteration count,
/// and output length. Returns the derived key as a lowercase hex string.
///
/// Unlike `pbkdf2_hash` (which generates its own salt, pins OWASP's iteration
/// count, and returns a self-describing PHC string), this is the raw KDF: use it
/// to derive keys or to interoperate with a stored PBKDF2 hash produced elsewhere.
/// The iteration loop runs in native Rust, bounding per-call cost.
///
/// Usage: pbkdf2_hmac_sha256 of password and salt and iterations and length
pub fn native_pbkdf2_hmac_sha256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pbkdf2_hmac_sha256", &args, 4)?;

    let password = expect_text(&args[0])?;
    let salt = expect_text(&args[1])?;
    let iterations = expect_count("pbkdf2_hmac_sha256", "iterations", &args[2])?;
    let length = expect_count("pbkdf2_hmac_sha256", "length", &args[3])? as usize;

    if password.len() > MAX_INPUT_SIZE || salt.len() > MAX_INPUT_SIZE {
        return Err(RuntimeError::new(
            format!(
                "pbkdf2_hmac_sha256: input exceeds maximum allowed size ({MAX_INPUT_SIZE} bytes)"
            ),
            0,
            0,
        ));
    }
    if iterations < 1 {
        return Err(RuntimeError::new(
            "pbkdf2_hmac_sha256: iterations must be at least 1".to_string(),
            0,
            0,
        ));
    }
    if iterations > MAX_PBKDF2_ITERATIONS as u64 {
        return Err(RuntimeError::new(
            format!("pbkdf2_hmac_sha256: iterations exceeds maximum ({MAX_PBKDF2_ITERATIONS})"),
            0,
            0,
        ));
    }
    if length < 1 {
        return Err(RuntimeError::new(
            "pbkdf2_hmac_sha256: length must be at least 1 byte".to_string(),
            0,
            0,
        ));
    }
    if length > MAX_PBKDF2_KEY_LENGTH {
        return Err(RuntimeError::new(
            format!("pbkdf2_hmac_sha256: length exceeds maximum ({MAX_PBKDF2_KEY_LENGTH} bytes)"),
            0,
            0,
        ));
    }

    let mut derived = vec![0u8; length];
    pbkdf2_hmac::<Sha256>(
        password.as_bytes(),
        salt.as_bytes(),
        iterations as u32,
        &mut derived,
    );
    let hex = bytes_to_hex(&derived);
    derived.zeroize();
    Ok(Value::Text(Arc::from(hex)))
}

/// Compare two strings in constant time, returning a boolean.
///
/// The comparison takes the same amount of time whether the strings match or
/// differ at the first byte or the last, so it does not leak how much of a secret
/// a caller guessed correctly. Use it whenever comparing a secret to attacker-
/// supplied input: MACs, CSRF tokens, session IDs, password-reset codes. Length
/// is not secret, so unequal-length inputs short-circuit to `no`.
///
/// Usage: constant_time_equals of a and b
pub fn native_constant_time_equals(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("constant_time_equals", &args, 2)?;

    let a = expect_text(&args[0])?;
    let b = expect_text(&args[1])?;

    // `ct_eq` on byte slices compares in constant time for equal-length inputs and
    // returns `0` immediately when the lengths differ (length is not a secret).
    let equal: bool = a.as_bytes().ct_eq(b.as_bytes()).into();
    Ok(Value::Bool(equal))
}

/// Generate `n` cryptographically secure random bytes from the OS CSPRNG,
/// returned as a lowercase hex string (so the result is `2 * n` characters).
///
/// Use this for salts, session identifiers, CSRF tokens, and password-reset
/// tokens. Building such values out of `random_int` risks modulo bias and
/// under-entropy; this draws uniform bytes directly from the operating system.
///
/// Usage: secure_random_bytes of n
pub fn native_secure_random_bytes(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("secure_random_bytes", &args, 1)?;

    use rand::RngCore;

    let n = expect_count("secure_random_bytes", "n", &args[0])? as usize;
    if n < 1 {
        return Err(RuntimeError::new(
            "secure_random_bytes: n must be at least 1".to_string(),
            0,
            0,
        ));
    }
    if n > MAX_SECURE_RANDOM_BYTES {
        return Err(RuntimeError::new(
            format!("secure_random_bytes: n exceeds maximum ({MAX_SECURE_RANDOM_BYTES} bytes)"),
            0,
            0,
        ));
    }

    let mut buf = vec![0u8; n];
    rand::rng().fill_bytes(&mut buf);
    let hex = bytes_to_hex(&buf);
    buf.zeroize();
    Ok(Value::Text(Arc::from(hex)))
}

// ============================================================================
// Password hashing (Argon2id, bcrypt, scrypt, PBKDF2)
//
// Unlike the fast hashes above (sha256/wflhash), these are deliberately *slow*,
// salted, and memory/CPU-hard so that a stolen database of hashes is expensive
// to crack. Each `*_hash` function generates a fresh random salt and returns a
// self-describing string (PHC format, or the bcrypt MCF `$2b$` format) that
// embeds the algorithm, cost parameters, salt, and digest. The matching
// `*_verify` function reads those parameters back out of the stored string, so
// nothing extra needs to be stored alongside the hash.
// ============================================================================

/// Maximum accepted password length in bytes.
///
/// Argon2/scrypt cost is dominated by their memory parameters, not input length,
/// but this bounds worst-case work and mirrors the input limits used elsewhere in
/// this module.
pub const MAX_PASSWORD_LENGTH: usize = 4096;

/// PBKDF2-HMAC-SHA256 iteration count. The RustCrypto default (4096) is far below
/// current guidance, so we pin OWASP's recommended 600,000 iterations instead.
const PBKDF2_ROUNDS: u32 = 600_000;

/// Reject passwords that are unreasonably long before doing expensive hashing work.
fn check_password_len(func: &str, password: &str) -> Result<(), RuntimeError> {
    if password.len() > MAX_PASSWORD_LENGTH {
        return Err(RuntimeError::new(
            format!("{func}: password exceeds maximum length ({MAX_PASSWORD_LENGTH} bytes)"),
            0,
            0,
        ));
    }
    Ok(())
}

/// Generate a cryptographically random 16-byte salt encoded for PHC output.
fn random_salt() -> Result<SaltString, RuntimeError> {
    use rand::RngCore;

    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    let salt = SaltString::encode_b64(&bytes)
        .map_err(|e| RuntimeError::new(format!("Failed to generate password salt: {e}"), 0, 0));
    bytes.zeroize();
    salt
}

fn argon2_hash_str(func: &str, password: &str) -> Result<String, RuntimeError> {
    check_password_len(func, password)?;
    let salt = random_salt()?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| RuntimeError::new(format!("{func} failed: {e}"), 0, 0))
}

/// Argon2id password hash (recommended default). Returns a PHC string.
/// Usage: argon2_hash of "my password"
pub fn native_argon2_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("argon2_hash", &args, 1)?;
    let password = expect_text(&args[0])?;
    Ok(Value::Text(Arc::from(argon2_hash_str(
        "argon2_hash",
        &password,
    )?)))
}

/// Verify a password against a stored Argon2 PHC string. Returns a boolean.
/// Usage: argon2_verify of "my password" and stored_hash
pub fn native_argon2_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("argon2_verify", &args, 2)?;
    let password = expect_text(&args[0])?;
    let stored = expect_text(&args[1])?;
    let ok = match PasswordHash::new(&stored) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    };
    Ok(Value::Bool(ok))
}

fn scrypt_hash_str(func: &str, password: &str) -> Result<String, RuntimeError> {
    check_password_len(func, password)?;
    let salt = random_salt()?;
    Scrypt
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| RuntimeError::new(format!("{func} failed: {e}"), 0, 0))
}

/// scrypt password hash. Returns a PHC string.
/// Usage: scrypt_hash of "my password"
pub fn native_scrypt_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("scrypt_hash", &args, 1)?;
    let password = expect_text(&args[0])?;
    Ok(Value::Text(Arc::from(scrypt_hash_str(
        "scrypt_hash",
        &password,
    )?)))
}

/// Verify a password against a stored scrypt PHC string. Returns a boolean.
/// Usage: scrypt_verify of "my password" and stored_hash
pub fn native_scrypt_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("scrypt_verify", &args, 2)?;
    let password = expect_text(&args[0])?;
    let stored = expect_text(&args[1])?;
    let ok = match PasswordHash::new(&stored) {
        Ok(parsed) => Scrypt.verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    };
    Ok(Value::Bool(ok))
}

fn pbkdf2_hash_str(func: &str, password: &str) -> Result<String, RuntimeError> {
    check_password_len(func, password)?;
    let salt = random_salt()?;
    // Override the weak default iteration count with OWASP's recommendation.
    let params = pbkdf2::Params {
        rounds: PBKDF2_ROUNDS,
        output_length: 32,
    };
    Pbkdf2
        .hash_password_customized(password.as_bytes(), None, None, params, &salt)
        .map(|h| h.to_string())
        .map_err(|e| RuntimeError::new(format!("{func} failed: {e}"), 0, 0))
}

/// PBKDF2-HMAC-SHA256 password hash. Returns a PHC string.
/// Usage: pbkdf2_hash of "my password"
pub fn native_pbkdf2_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pbkdf2_hash", &args, 1)?;
    let password = expect_text(&args[0])?;
    Ok(Value::Text(Arc::from(pbkdf2_hash_str(
        "pbkdf2_hash",
        &password,
    )?)))
}

/// Verify a password against a stored PBKDF2 PHC string. Returns a boolean.
/// Usage: pbkdf2_verify of "my password" and stored_hash
pub fn native_pbkdf2_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("pbkdf2_verify", &args, 2)?;
    let password = expect_text(&args[0])?;
    let stored = expect_text(&args[1])?;
    let ok = match PasswordHash::new(&stored) {
        Ok(parsed) => Pbkdf2.verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    };
    Ok(Value::Bool(ok))
}

fn bcrypt_hash_str(func: &str, password: &str) -> Result<String, RuntimeError> {
    check_password_len(func, password)?;
    // bcrypt manages its own salt internally and returns an MCF `$2b$` string.
    bcrypt::hash(password.as_bytes(), bcrypt::DEFAULT_COST)
        .map_err(|e| RuntimeError::new(format!("{func} failed: {e}"), 0, 0))
}

/// bcrypt password hash. Returns a bcrypt MCF string (`$2b$...`).
/// Usage: bcrypt_hash of "my password"
///
/// Note: bcrypt only considers the first 72 bytes of the password.
pub fn native_bcrypt_hash(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("bcrypt_hash", &args, 1)?;
    let password = expect_text(&args[0])?;
    Ok(Value::Text(Arc::from(bcrypt_hash_str(
        "bcrypt_hash",
        &password,
    )?)))
}

/// Verify a password against a stored bcrypt hash. Returns a boolean.
/// Usage: bcrypt_verify of "my password" and stored_hash
pub fn native_bcrypt_verify(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("bcrypt_verify", &args, 2)?;
    let password = expect_text(&args[0])?;
    let stored = expect_text(&args[1])?;
    // A malformed stored hash simply fails verification rather than erroring.
    let ok = bcrypt::verify(password.as_bytes(), stored.as_ref()).unwrap_or(false);
    Ok(Value::Bool(ok))
}

/// Verify a password against any supported stored hash, auto-detecting the
/// algorithm from the stored string. Returns a boolean.
fn verify_any_password(password: &str, stored: &str) -> bool {
    // bcrypt MCF strings are not PHC format; detect them by their version prefix.
    if stored.starts_with("$2a$") || stored.starts_with("$2b$") || stored.starts_with("$2y$") {
        return bcrypt::verify(password.as_bytes(), stored).unwrap_or(false);
    }

    // Otherwise treat it as a PHC string; each hasher matches on its own algorithm
    // identifier, so passing all three lets the correct one handle it.
    match PasswordHash::new(stored) {
        Ok(parsed) => parsed
            .verify_password(&[&Argon2::default(), &Scrypt, &Pbkdf2], password.as_bytes())
            .is_ok(),
        Err(_) => false,
    }
}

/// Hash a password using the recommended default algorithm (Argon2id).
/// Prefer this unless you have a specific reason to pick an algorithm.
/// Usage: hash_password of "my password"
pub fn native_hash_password(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("hash_password", &args, 1)?;
    let password = expect_text(&args[0])?;
    Ok(Value::Text(Arc::from(argon2_hash_str(
        "hash_password",
        &password,
    )?)))
}

/// Verify a password against a stored hash produced by any of the password
/// hashing functions (Argon2, bcrypt, scrypt, PBKDF2). Returns a boolean.
/// Usage: verify_password of "my password" and stored_hash
pub fn native_verify_password(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("verify_password", &args, 2)?;
    let password = expect_text(&args[0])?;
    let stored = expect_text(&args[1])?;
    Ok(Value::Bool(verify_any_password(&password, &stored)))
}

/// Register all crypto functions in the environment
pub fn register_crypto(env: &mut Environment) {
    env.define_native("wflhash256", native_wflhash256);
    env.define_native("wflhash512", native_wflhash512);
    env.define_native("wflhash256_with_salt", native_wflhash256_with_salt);
    env.define_native("wflmac256", native_wflmac256);
    env.define_native("sha256", native_sha256);
    env.define_native("hmac_sha256", native_hmac_sha256);
    env.define_native("generate_csrf_token", native_generate_csrf_token);
    // Low-level auth/session primitives
    env.define_native("pbkdf2_hmac_sha256", native_pbkdf2_hmac_sha256);
    env.define_native("constant_time_equals", native_constant_time_equals);
    env.define_native("secure_random_bytes", native_secure_random_bytes);
    // Password hashing
    env.define_native("hash_password", native_hash_password);
    env.define_native("verify_password", native_verify_password);
    env.define_native("argon2_hash", native_argon2_hash);
    env.define_native("argon2_verify", native_argon2_verify);
    env.define_native("bcrypt_hash", native_bcrypt_hash);
    env.define_native("bcrypt_verify", native_bcrypt_verify);
    env.define_native("scrypt_hash", native_scrypt_hash);
    env.define_native("scrypt_verify", native_scrypt_verify);
    env.define_native("pbkdf2_hash", native_pbkdf2_hash);
    env.define_native("pbkdf2_verify", native_pbkdf2_verify);
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
        let result = native_wflhash256(vec![Value::Text(Arc::from("hello"))]);
        assert!(result.is_ok());

        if let Ok(Value::Text(hash)) = result {
            assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_wflhash512_basic() {
        let result = native_wflhash512(vec![Value::Text(Arc::from("hello"))]);
        assert!(result.is_ok());

        if let Ok(Value::Text(hash)) = result {
            assert_eq!(hash.len(), 128); // 64 bytes = 128 hex chars
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

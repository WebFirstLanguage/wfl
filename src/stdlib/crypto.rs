use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use hkdf::Hkdf;
use sha2::Sha256;
use std::rc::Rc;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

/// Number of rounds in WFLHASH-P permutation
const WFLHASH_ROUNDS: usize = 24;

/// Maximum input size (100MB) to prevent DoS
pub const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// WFLHASH IVs (Cube roots of primes 2, 3, 5, 7)
const WFLHASH_IV: [[u64; 4]; 4] = [
    [
        0x428a2f98d728ae22,
        0x7137449123ef65cd,
        0xb5c0fbcfec4d3b2f,
        0xe9b5dba58189dbbc,
    ],
    [
        0x3956c25bf348b538,
        0x59f111f1b605d019,
        0x923f82a4af194f9b,
        0xab1c5ed5da6d8118,
    ],
    [
        0xd807aa98a3030242,
        0x12835b0145706fbe,
        0x243185be4ee4b28c,
        0x550c7dc3d5ffb4e2,
    ],
    [
        0x72be5d74f27b896f,
        0x80deb1fe3b1696b1,
        0x9bdc06a725c71235,
        0xc19bf174cf692694,
    ],
];

/// Strong round constants (Cube roots of primes)
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

#[derive(Clone, Debug)]
struct WflHashState {
    state: [[u64; 4]; 4],
    /// Buffer to handle sponge construction correctly
    buffer: [u8; 64],
    buffer_len: usize,
    /// Total length processed (in bytes) for padding
    total_len: u128,
}

impl Drop for WflHashState {
    fn drop(&mut self) {
        self.state.zeroize();
        self.buffer.zeroize();
    }
}

impl WflHashState {
    fn new() -> Self {
        Self {
            state: [[0u64; 4]; 4],
            buffer: [0u8; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    fn initialize(&mut self, params: &WflHashParams) {
        self.state = WFLHASH_IV;

        // Secure parameter mixing
        self.state[0][0] ^= params.digest_length as u64;
        self.state[0][1] ^= params.key_length as u64;
        self.state[0][2] ^= params.mode_flags as u64;

        for (i, &byte) in params.personalization.iter().enumerate() {
            let word_idx = i / 8;
            let byte_idx = i % 8;
            if word_idx < 2 {
                self.state[0][word_idx + 2] ^= (byte as u64) << (byte_idx * 8);
            }
        }

        self.permute();

        // Absorb derived key if in MAC mode
        if (params.mode_flags & 0x01) != 0 {
            self.absorb_bytes(&params.derived_key);
        }
    }

    /// The WFLHASH-P Permutation
    fn permute(&mut self) {
        for (_round, &round_constant) in ROUND_CONSTANTS.iter().enumerate().take(WFLHASH_ROUNDS) {
            self.state[0][0] ^= round_constant;

            // Column G-function
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

            // Row G-function
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

    /// Fixed G-function: Invertible and Efficient
    /// Removed black_box (performance killer)
    /// Fixed rotation constants for 64-bit words
    /// Fixed mixing step to be sequential (Feistel) to guarantee invertibility
    #[inline(always)]
    fn g_function(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
        // Standard ARX Quarter Round (ChaCha/BLAKE structure)
        // Rotations optimized for 64-bit diffusion: 32, 24, 16, 63
        // 63 replaces 14 because 14/16 are too close. 63 gives neighbor-bit diffusion.

        *a = a.wrapping_add(*b);
        *d = (*d ^ *a).rotate_right(32);

        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_right(24);

        *a = a.wrapping_add(*b);
        *d = (*d ^ *a).rotate_right(16);

        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_right(63);

        // Enhanced Mixing - Fixed to be Sequential/Reversible
        // Previous parallel version was NOT invertible (Det = 0)
        *a ^= c.rotate_left(13);
        *c ^= a.rotate_left(7);
    }

    /// Absorb logic that buffers input correctly
    fn absorb_bytes(&mut self, data: &[u8]) {
        let mut pos = 0;
        let len = data.len();

        while pos < len {
            let space = 64 - self.buffer_len;
            let copy_len = space.min(len - pos);

            self.buffer[self.buffer_len..self.buffer_len + copy_len]
                .copy_from_slice(&data[pos..pos + copy_len]);

            self.buffer_len += copy_len;
            pos += copy_len;
            self.total_len += copy_len as u128;

            // If buffer is full, process it
            if self.buffer_len == 64 {
                self.process_buffer_block();
                self.buffer_len = 0;
            }
        }
    }

    /// Process a single full 64-byte block from buffer
    fn process_buffer_block(&mut self) {
        for (i, chunk_u64) in self.buffer.chunks(8).enumerate() {
            let value = u64::from_le_bytes(chunk_u64.try_into().unwrap());
            let row = i / 4;
            let col = i % 4;
            self.state[row][col] ^= value;
        }
        self.permute();
    }

    /// Finalize: Apply Padding and Squeeze
    fn finalize(&mut self, digest_length: usize) -> Vec<u8> {
        // 1. Append 0x80
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        // 2. If not enough space for length (needs 16 bytes for u128),
        // pad with zeros, process, and start new block.
        // We use u128 for total length (16 bytes).
        if self.buffer_len > 48 {
            // 64 - 16 = 48
            // Pad remainder of this block with zeros
            while self.buffer_len < 64 {
                self.buffer[self.buffer_len] = 0;
                self.buffer_len += 1;
            }
            self.process_buffer_block();
            self.buffer_len = 0;
        }

        // 3. Pad zeros until length position
        while self.buffer_len < 48 {
            self.buffer[self.buffer_len] = 0;
            self.buffer_len += 1;
        }

        // 4. Append length in bits (u128 little endian)
        let bit_len = self.total_len * 8;
        let len_bytes = bit_len.to_le_bytes();
        self.buffer[48..64].copy_from_slice(&len_bytes);

        // 5. Process final padded block
        self.process_buffer_block();

        // 6. Squeeze
        self.squeeze(digest_length)
    }

    fn squeeze(&mut self, output_bytes: usize) -> Vec<u8> {
        let mut output = Vec::with_capacity(output_bytes);

        while output.len() < output_bytes {
            let rate_words = [
                self.state[0][0],
                self.state[0][1],
                self.state[0][2],
                self.state[0][3],
                self.state[1][0],
                self.state[1][1],
                self.state[1][2],
                self.state[1][3],
            ];

            for &word in &rate_words {
                output.extend_from_slice(&word.to_le_bytes());
                if output.len() >= output_bytes {
                    break;
                }
            }

            if output.len() < output_bytes {
                self.permute();
            }
        }
        output
    }
}

// ... Params struct remains mostly the same, ensuring zeroize ...
#[derive(Clone, Debug)]
struct WflHashParams {
    digest_length: usize,
    key_length: usize,
    mode_flags: u32,
    personalization: [u8; 16],
    derived_key: [u8; 64],
}

impl Drop for WflHashParams {
    fn drop(&mut self) {
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

    fn new_with_personalization(digest_length: usize, personal: &[u8]) -> Self {
        let mut params = Self::new(digest_length);
        let copy_len = personal.len().min(16);
        params.personalization[..copy_len].copy_from_slice(&personal[..copy_len]);
        params.mode_flags |= 0x02;
        params
    }

    fn new_with_key(digest_length: usize, key: &[u8]) -> Result<Self, RuntimeError> {
        let mut params = Self::new(digest_length);
        let hkdf = Hkdf::<Sha256>::new(None, key);
        let info = b"WFLMAC-256-KEY-DERIVATION";

        match hkdf.expand(info, &mut params.derived_key) {
            Ok(_) => {
                params.key_length = key.len();
                params.mode_flags |= 0x01;
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

// Core functions rewritten to use the buffered State logic

fn wflhash_core(input: &[u8], params: &WflHashParams) -> Result<Vec<u8>, RuntimeError> {
    if input.len() > MAX_INPUT_SIZE {
        return Err(RuntimeError::new(
            "Input exceeds maximum allowed size".to_string(),
            0,
            0,
        ));
    }
    let mut state = WflHashState::new();
    state.initialize(params);
    state.absorb_bytes(input);
    Ok(state.finalize(params.digest_length))
}

fn wflhash_core_text(input: &[u8], params: &WflHashParams) -> Result<Vec<u8>, RuntimeError> {
    if std::str::from_utf8(input).is_err() {
        return Err(RuntimeError::new("Invalid text encoding".to_string(), 0, 0));
    }
    wflhash_core(input, params)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// Native functions exposed to interpreter (API kept consistent)

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
        _ => return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0)),
    };

    let params = WflHashParams::new(32);
    let hash = wflhash_core_text(input, &params)?;
    Ok(Value::Text(Rc::from(bytes_to_hex(&hash))))
}

pub fn native_wflhash256_binary(input: &[u8]) -> Result<String, RuntimeError> {
    let params = WflHashParams::new(32);
    let hash = wflhash_core(input, &params)?; // valid because wflhash_core checks size and doesn't check specific encoding
    Ok(bytes_to_hex(&hash))
}

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
        _ => return Err(RuntimeError::new("Invalid argument type".to_string(), 0, 0)),
    };

    let params = WflHashParams::new(64);
    let hash = wflhash_core_text(input, &params)?;
    Ok(Value::Text(Rc::from(bytes_to_hex(&hash))))
}

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
        _ => return Err(RuntimeError::new("Invalid arg type".to_string(), 0, 0)),
    };
    let salt = match &args[1] {
        Value::Text(text) => text.as_bytes(),
        _ => return Err(RuntimeError::new("Invalid arg type".to_string(), 0, 0)),
    };

    let params = WflHashParams::new_with_personalization(32, salt);
    let hash = wflhash_core_text(input, &params)?;
    Ok(Value::Text(Rc::from(bytes_to_hex(&hash))))
}

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
        _ => return Err(RuntimeError::new("Invalid arg type".to_string(), 0, 0)),
    };
    let key = match &args[1] {
        Value::Text(text) => text.as_bytes(),
        _ => return Err(RuntimeError::new("Invalid arg type".to_string(), 0, 0)),
    };

    let params = WflHashParams::new_with_key(32, key)?;
    let hash = wflhash_core_text(input, &params)?;
    Ok(Value::Text(Rc::from(bytes_to_hex(&hash))))
}

// Verification function (kept subtle constant time check)
pub fn wflmac256_verify(
    message: &[u8],
    key: &[u8],
    expected_mac: &str,
) -> Result<bool, RuntimeError> {
    let params = WflHashParams::new_with_key(32, key)?;
    let computed = wflhash_core(message, &params)?;
    let computed_hex = bytes_to_hex(&computed);
    if expected_mac.len() != 64 {
        return Ok(false);
    }
    Ok(computed_hex
        .as_bytes()
        .ct_eq(expected_mac.as_bytes())
        .into())
}

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

use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::rc::Rc;

/// WFLHASH internal state - 1024 bits organized as 4x4 matrix of u64
#[derive(Clone, Debug)]
struct WflHashState {
    state: [[u64; 4]; 4],
}

impl WflHashState {
    /// Create a new WFLHASH state initialized to zero
    fn new() -> Self {
        Self {
            state: [[0u64; 4]; 4],
        }
    }

    /// Initialize state with parameter block
    fn initialize(&mut self, params: &WflHashParams) {
        // Initialize state with parameter block values
        // This is a simplified initialization - full spec would be more complex
        self.state[0][0] = params.digest_length as u64;
        self.state[0][1] = params.key_length as u64;
        self.state[0][2] = params.mode_flags as u64;
        self.state[0][3] = 0x6A09E667F3BCC908u64; // SHA-2 constant as placeholder

        // Fill remaining state with constants (simplified)
        for i in 1..4 {
            for j in 0..4 {
                self.state[i][j] = 0x243F6A8885A308D3u64.wrapping_add((i * 4 + j) as u64);
            }
        }
    }

    /// Apply WFLHASH-P permutation function
    fn permute(&mut self) {
        // Simplified WFLHASH-P permutation - 12 rounds
        for round in 0..12 {
            // Add round constant
            self.state[0][0] = self.state[0][0].wrapping_add(round as u64);

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

    /// G function - ARX operations (Add-Rotate-XOR) inspired by ChaCha
    fn g_function(a: &mut u64, b: &mut u64, c: &mut u64, d: &mut u64) {
        // First quarter-round
        *a = a.wrapping_add(*b);
        *d = (*d ^ *a).rotate_right(32);

        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_right(24);

        // Second quarter-round
        *a = a.wrapping_add(*b);
        *d = (*d ^ *a).rotate_right(16);

        *c = c.wrapping_add(*d);
        *b = (*b ^ *c).rotate_right(63);
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

    /// Absorb data into the sponge
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

/// WFLHASH parameter block
#[derive(Clone, Debug)]
struct WflHashParams {
    digest_length: usize,
    key_length: usize,
    mode_flags: u32,
    #[allow(dead_code)] // Reserved for future use
    personalization: [u8; 16],
}

impl WflHashParams {
    fn new(digest_length: usize) -> Self {
        Self {
            digest_length,
            key_length: 0,
            mode_flags: 0,
            personalization: [0u8; 16],
        }
    }
}

/// Core WFLHASH function
fn wflhash_core(input: &[u8], params: &WflHashParams) -> Vec<u8> {
    let mut state = WflHashState::new();
    state.initialize(params);

    // Absorb input
    state.absorb(input);

    // Add padding (simplified - real implementation would be more complex)
    let padding = [0x80u8]; // Simple padding
    state.absorb(&padding);

    // Squeeze output
    state.squeeze(params.digest_length)
}

/// Convert bytes to hexadecimal string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// WFLHASH-256 implementation
pub fn native_wflhash256(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("wflhash256 expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let input = match &args[0] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new(
                format!("wflhash256 expects text input, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    let params = WflHashParams::new(32); // 256 bits = 32 bytes
    let hash_bytes = wflhash_core(input, &params);
    let hash_hex = bytes_to_hex(&hash_bytes);

    Ok(Value::Text(Rc::from(hash_hex)))
}

/// WFLHASH-512 implementation
pub fn native_wflhash512(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("wflhash512 expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let input = match &args[0] {
        Value::Text(text) => text.as_bytes(),
        _ => {
            return Err(RuntimeError::new(
                format!("wflhash512 expects text input, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    let params = WflHashParams::new(64); // 512 bits = 64 bytes
    let hash_bytes = wflhash_core(input, &params);
    let hash_hex = bytes_to_hex(&hash_bytes);

    Ok(Value::Text(Rc::from(hash_hex)))
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

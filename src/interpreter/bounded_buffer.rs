use std::collections::VecDeque;

/// A ring buffer with a maximum size to prevent unbounded memory growth
/// When the buffer is full, oldest data is dropped to make room for new data
#[derive(Debug)]
pub struct BoundedBuffer {
    data: VecDeque<u8>,
    max_size: usize,
    bytes_written: usize,
    bytes_dropped: usize,
}

impl BoundedBuffer {
    /// Create a new bounded buffer with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_size.min(8192)),
            max_size,
            bytes_written: 0,
            bytes_dropped: 0,
        }
    }

    /// Push bytes into the buffer. If the buffer is full, oldest bytes are dropped.
    pub fn push(&mut self, bytes: &[u8]) {
        self.bytes_written += bytes.len();

        for &byte in bytes {
            if self.data.len() >= self.max_size {
                self.data.pop_front();
                self.bytes_dropped += 1;
            }
            self.data.push_back(byte);
        }
    }

    /// Read all data from the buffer and clear it
    pub fn read_all(&mut self) -> Vec<u8> {
        let result = self.data.iter().copied().collect();
        self.data.clear();
        result
    }

    /// Get the current size of the buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get statistics about the buffer
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            current_size: self.data.len(),
            max_size: self.max_size,
            bytes_written: self.bytes_written,
            bytes_dropped: self.bytes_dropped,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BufferStats {
    pub current_size: usize,
    pub max_size: usize,
    pub bytes_written: usize,
    pub bytes_dropped: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_buffer_basic() {
        let mut buf = BoundedBuffer::new(10);

        buf.push(b"hello");
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.bytes_dropped, 0);

        let data = buf.read_all();
        assert_eq!(data, b"hello");
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_bounded_buffer_overflow() {
        let mut buf = BoundedBuffer::new(10);

        // Write 15 bytes (5 should be dropped)
        buf.push(b"0123456789ABCDE");

        assert_eq!(buf.len(), 10);
        assert_eq!(buf.bytes_dropped, 5);

        let data = buf.read_all();
        // Should contain last 10 bytes: "56789ABCDE"
        assert_eq!(data, b"56789ABCDE");
    }

    #[test]
    fn test_bounded_buffer_multiple_pushes() {
        let mut buf = BoundedBuffer::new(8);

        buf.push(b"AAAA");
        assert_eq!(buf.len(), 4);

        buf.push(b"BBBB");
        assert_eq!(buf.len(), 8);

        buf.push(b"CCCC");
        // Should drop first 4 bytes (AAAA) to make room for CCCC
        assert_eq!(buf.len(), 8);
        assert_eq!(buf.bytes_dropped, 4);

        let data = buf.read_all();
        assert_eq!(data, b"BBBBCCCC");
    }

    #[test]
    fn test_bounded_buffer_stats() {
        let mut buf = BoundedBuffer::new(5);

        buf.push(b"0123456789");

        let stats = buf.stats();
        assert_eq!(stats.current_size, 5);
        assert_eq!(stats.max_size, 5);
        assert_eq!(stats.bytes_written, 10);
        assert_eq!(stats.bytes_dropped, 5);
    }

    #[test]
    fn test_bounded_buffer_empty() {
        let buf = BoundedBuffer::new(10);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_bounded_buffer_large_single_write() {
        let mut buf = BoundedBuffer::new(100);

        let large_data = vec![b'X'; 500];
        buf.push(&large_data);

        // Should only keep last 100 bytes
        assert_eq!(buf.len(), 100);
        assert_eq!(buf.bytes_dropped, 400);

        let data = buf.read_all();
        assert_eq!(data.len(), 100);
        assert!(data.iter().all(|&b| b == b'X'));
    }
}

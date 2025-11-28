
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_buf: u32,
    bit_count: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte_pos: 0, bit_buf: 0, bit_count: 0 }
    }

    // Fill the buffer to ensure we have at least `need` bits in bit_buf
    fn ensure_bits(&mut self, need: u8) -> bool {
        while self.bit_count < need {
            if self.byte_pos >= self.data.len() {
                return false;
            }
            let b = self.data[self.byte_pos] as u32;
            self.byte_pos += 1;
            self.bit_buf |= b << self.bit_count;
            self.bit_count += 8;
        }
        true
    }

    pub fn read_bits(&mut self, n: usize) -> Option<u32> {
        if n == 0 { return Some(0); }
        if n > 32 { return None; }
        if !self.ensure_bits(n as u8) { return None; }
        let mask = if n == 32 { !0u32 } else { (1u32 << n) - 1 };
        let bits = self.bit_buf & mask;
        self.bit_buf >>= n;
        self.bit_count -= n as u8;
        Some(bits)
    }

    /// Peek bits without consuming. Returns lowest `n` bits, if available
    pub fn peek_bits(&mut self, n: usize) -> Option<u32> {
        if n == 0 { return Some(0); }
        if n > 32 { return None; }
        if !self.ensure_bits(n as u8) { return None; }
        let mask = if n == 32 { !0u32 } else { (1u32 << n) - 1 };
        Some(self.bit_buf & mask)
    }

    pub fn align_to_byte(&mut self) {
        let skip = (self.bit_count % 8) as u32;
        if skip > 0 {
            self.read_bits(skip as usize);
        }
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        // if we have a partially consumed bitbuf, drain lowest bits until byte aligned
        if self.bit_count >= 8 {
            let b = (self.bit_buf & 0xff) as u8;
            self.bit_buf >>= 8;
            self.bit_count -= 8;
            return Some(b);
        }
        // not enough buffered; but we must be byte-aligned for stored block reads
        if self.bit_count != 0 {
            // This should not happen because caller should call align_to_byte first
            return None;
        }
        if self.byte_pos >= self.data.len() { return None; }
        let b = self.data[self.byte_pos];
        self.byte_pos += 1;
        Some(b)
    }
}

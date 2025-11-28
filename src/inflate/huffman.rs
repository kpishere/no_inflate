use alloc::vec::Vec;
use core::convert::TryInto;
use crate::inflate::bitreader::BitReader;
use crate::InflateError;

pub struct HuffmanTable {
    max_bits: usize,
    table: Vec<u32>, // packed as (symbol << 8) | len, 0xffffffff for invalid
}

fn reverse_bits(mut v: u32, len: usize) -> u32 {
    let mut r = 0u32;
    for _ in 0..len {
        r = (r << 1) | (v & 1);
        v >>= 1;
    }
    r
}

impl HuffmanTable {
    pub fn from_lengths(lengths: &[u8]) -> Result<Self, InflateError> {
        let mut max_bits = 0usize;
        for &l in lengths.iter() {
            if l as usize > max_bits { max_bits = l as usize; }
        }
        if max_bits == 0 { max_bits = 1; }
        if max_bits > 15 {
            // We'll cap at 15
            max_bits = 15;
        }

        // Count codes per length
        let mut counts = vec![0u16; max_bits + 1];
        for &l in lengths.iter() {
            if l as usize > max_bits { return Err(InflateError::BadHuffmanCode); }
            if l > 0 { counts[l as usize] += 1; }
        }
        // compute next_code
        let mut next_code = vec![0u32; max_bits + 1];
        let mut code = 0u32;
        for bits in 1..=max_bits {
            code = (code + counts[bits - 1] as u32) << 1;
            next_code[bits] = code;
        }

        let table_size = 1usize << max_bits;
        let mut table = vec![0xffffffffu32; table_size];

        for (sym, &len) in lengths.iter().enumerate() {
            let len = len as usize;
            if len == 0 { continue; }
            let code = next_code[len];
            next_code[len] += 1;
            let rev = reverse_bits(code, len);
            let shift = max_bits - len;
            let start = (rev as usize) << shift;
            let end = start + (1usize << shift);
            let packed = ((sym as u32) << 8) | (len as u32);
            for idx in start..end {
                table[idx] = packed;
            }
        }

        Ok(HuffmanTable { max_bits, table })
    }

    pub fn read_symbol(&self, br: &mut BitReader) -> Result<u16, InflateError> {
        // peek max_bits
        let peek = br.peek_bits(self.max_bits).ok_or(InflateError::InputTooShort)?;
        let idx = peek as usize;
        let entry = self.table.get(idx).ok_or(InflateError::BadHuffmanCode)?;
        if *entry == 0xffffffffu32 { return Err(InflateError::BadHuffmanCode); }
        let sym = (*entry >> 8) as u16;
        let len = (*entry & 0xff) as usize;
        // consume bits
        let _ = br.read_bits(len).ok_or(InflateError::InputTooShort)?;
        Ok(sym)
    }
}

// build fixed tables for litlen and dist

pub fn build_fixed_litlen_table() -> HuffmanTable {
    // per RFC 1951
    // litlen lengths: 0-143:8 bits, 144-255:9, 256-279:7, 280-287:8
    let mut lengths = vec![0u8; 288];
    for i in 0..=143 { lengths[i] = 8; }
    for i in 144..=255 { lengths[i] = 9; }
    for i in 256..=279 { lengths[i] = 7; }
    for i in 280..=287 { lengths[i] = 8; }
    HuffmanTable::from_lengths(&lengths).expect("failed to build fixed litlen table")
}

pub fn build_fixed_dist_table() -> HuffmanTable {
    let lengths = vec![5u8; 32];
    HuffmanTable::from_lengths(&lengths).expect("failed to build fixed dist table")
}

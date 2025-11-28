use alloc::vec::Vec;
use crate::inflate::bitreader::BitReader;
use crate::InflateError;

pub struct HuffmanTable {
    pub max_bits: usize,
    pub children: Vec<i32>,
    pub symbol: Vec<i32>,
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
        let mut counts: Vec<usize> = Vec::new();
        counts.resize(max_bits + 1, 0usize);
        for &l in lengths.iter() {
            if l as usize > max_bits { return Err(InflateError::BadHuffmanCode); }
            if l > 0 { counts[l as usize] += 1; }
        }
        // compute next_code
        let mut next_code: Vec<u32> = Vec::new();
        next_code.resize(max_bits + 1, 0u32);
        let mut code = 0u32;
        for bits in 1..=max_bits {
            code = (code + counts[bits - 1] as u32) << 1;
            next_code[bits] = code;
        }

        // build trie
        let mut children: Vec<i32> = Vec::new();
        let mut symbol_vec: Vec<i32> = Vec::new();
        children.push(-1); children.push(-1); symbol_vec.push(-1); // root

        for (sym, &l) in lengths.iter().enumerate() {
            let len = l as usize;
            if len == 0 { continue; }
            let c = next_code[len];
            next_code[len] += 1;
            let rev = reverse_bits(c, len);
            let mut node = 0usize;
            for i in 0..len {
                let bit = ((rev >> i) & 1) as usize;
                let child_idx = node * 2 + bit;
                if child_idx >= children.len() { children.resize(child_idx + 1, -1); }
                let child = children[child_idx];
                if child == -1 {
                    let new_node = symbol_vec.len() as i32;
                    children.push(-1); children.push(-1);
                    symbol_vec.push(-1);
                    children[child_idx] = new_node;
                    node = new_node as usize;
                } else {
                    node = child as usize;
                }
            }
            if symbol_vec[node] != -1 { return Err(InflateError::BadHuffmanCode); }
            symbol_vec[node] = sym as i32;
        }

        Ok(HuffmanTable { max_bits, children, symbol: symbol_vec })
    }

    pub fn read_symbol(&self, br: &mut BitReader) -> Result<u16, InflateError> {
        let mut node = 0usize;
        loop {
            if self.symbol[node] != -1 {
                return Ok(self.symbol[node] as u16);
            }
            let bit = br.read_bits(1).ok_or(InflateError::InputTooShort)? as usize;
            let child_idx = node * 2 + bit;
            if child_idx >= self.children.len() { return Err(InflateError::BadHuffmanCode); }
            let child = self.children[child_idx];
            if child == -1 { return Err(InflateError::BadHuffmanCode); }
            node = child as usize;
        }
    }

    // Debug helpers
    pub fn max_bits(&self) -> usize { self.max_bits }
    pub fn children(&self) -> &[i32] { &self.children }
    pub fn symbols(&self) -> &[i32] { &self.symbol }
}

// build fixed tables for litlen and dist

pub fn build_fixed_litlen_table() -> HuffmanTable {
    // per RFC 1951
    // litlen lengths: 0-143:8 bits, 144-255:9, 256-279:7, 280-287:8
    let mut lengths: Vec<u8> = Vec::new();
    lengths.resize(288, 0u8);
    for i in 0..=143 { lengths[i] = 8; }
    for i in 144..=255 { lengths[i] = 9; }
    for i in 256..=279 { lengths[i] = 7; }
    for i in 280..=287 { lengths[i] = 8; }
    HuffmanTable::from_lengths(&lengths).expect("failed to build fixed litlen table")
}

pub fn build_fixed_dist_table() -> HuffmanTable {
    let mut lengths = Vec::new();
    lengths.resize(32, 5u8);
    HuffmanTable::from_lengths(&lengths).expect("failed to build fixed dist table")
}

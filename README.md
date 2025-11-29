# no_inflate

[An AI generated library that is human verified.]

A minimal no_std Rust library implementing zlib inflate (DEFLATE) algorithm in pure Rust using `alloc` for dynamic output buffer.

This crate provides a `no_std`-compatible inflate implementation that reads compressed zlib input from a static byte slice and returns a dynamically allocated `Vec<u8>` as output.

Key features:
- no_std (uses `alloc` for dynamic buffers)
- Supports stored, fixed, and dynamic Huffman blocks (RFC 1951)
- No dependencies on the standard library for the library code

BEFORE USING:
Sorry, there is issue with how this lib is done and using 'cargo publish' feature.
Before using as no_std library, edit the file srd/lib.rs and uncomment the 3rd line.
(Publish operation appears to run a test but the test isn't including stdlib.)
```rust
//#![cfg_attr(not(test), no_std)]
```

Usage:
```rust
use no_inflate::inflate_zlib;

let compressed: &[u8] = /* zlib compressed bytes */;
let decompressed = inflate_zlib(compressed).expect("decompress");
// `decompressed` is a Vec<u8>
```

Run tests (requires a standard Rust toolchain):

```bash
cargo test --lib test
```

Limitations:
- Adler32 check is not implemented
- Preset dictionaries (FDICT) are not supported

Note about allocators:
The library is written as `no_std` and uses `alloc` for dynamic buffer allocation. When using this crate in `no_std` environments, ensure a global allocator is provided by your runtime or by selecting an allocator crate (for example `linked_list_allocator` or `wee_alloc`) and registering it as the `#[global_allocator]` in your platform.

Contributions welcome.

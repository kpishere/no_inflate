use no_inflate::inflate_zlib;

fn main() {
    let input = b"The quick brown fox jumps over the lazy dog";
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(input).expect("write input to compressor");
    let compressed = enc.finish().expect("finish compression");
    let hex: String = compressed.iter().map(|b| format!("{:02x}", b)).collect();
    println!("compressed hex: {}", hex);
    match inflate_zlib(&compressed) {
        Ok(out) => {
            println!("decompressed ok: {} bytes", out.len());
            println!("{}", String::from_utf8_lossy(&out));
        }
        Err(e) => println!("inflate error: {:?}", e),
    }
}

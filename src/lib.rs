#![cfg_attr(not(test), no_std)]
#![deny(warnings)]

extern crate alloc;

// expose the main API
pub mod inflate;

pub use inflate::{inflate_zlib, InflateError};

#[cfg(test)] 
mod tests {
    use crate::inflate_zlib;
    use std::process::Command;

    #[test]
    fn roundtrip_simple() {
        let input = b"The quick brown fox jumps over the lazy dog";
        // Compress with flate2 zlib
        use flate2::{Compression, write::ZlibEncoder};
        use std::io::Write;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(input).unwrap();
        let compressed = enc.finish().unwrap();

        let decompressed = inflate_zlib(&compressed).expect("decompress");
        assert_eq!(decompressed, input);
    }

    #[test]
    fn roundtrip_large() {
        let mut data = Vec::new();
        for _i in 0..1000 {
            data.extend_from_slice(b"Hello WORLD '123456' ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        }
        use flate2::{Compression, write::ZlibEncoder};
        use std::io::Write;
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&data).unwrap();
        let compressed = enc.finish().unwrap();

        let decompressed = inflate_zlib(&compressed).expect("decompress");
        assert_eq!(decompressed, data);
    }

    #[test]
    fn roundtrip_python_zlib() {
        let input = b"The quick brown fox jumps over the lazy dog";
        // base64 encode input to pass as an argument to python to avoid quoting issues
        let b64_input = base64::encode(input);

        let python_cmd = if Command::new("python3").arg("--version").output().is_ok() {
            "python3"
        } else {
            "python"
        };
        let script = "import sys, zlib, base64; inp = base64.b64decode(sys.argv[1]); out = zlib.compress(inp); sys.stdout.write(base64.b64encode(out).decode())";
        let out = Command::new(python_cmd)
            .arg("-c")
            .arg(script)
            .arg(&b64_input)
            .output()
            .expect("failed to run python to create compressed test data");
        assert!(out.status.success(), "python script failed: {}", String::from_utf8_lossy(&out.stderr));

        let b64_compressed = String::from_utf8(out.stdout).expect("python wrote non-utf8 output");
        let compressed = base64::decode(b64_compressed.trim()).expect("failed to decode base64 from python");

        let decompressed = inflate_zlib(&compressed).expect("decompress");
        assert_eq!(decompressed, input);
    }
}

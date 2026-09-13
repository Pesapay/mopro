use crate::init::adapter::Adapter;
use crate::init::proving_system::ProvingSystem;
use include_dir::include_dir;
use include_dir::Dir;

pub struct Speakup;

impl ProvingSystem for Speakup {
    const TEMPLATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/template/speakup");

    const ADAPTER: Adapter = Adapter::Speakup;

    // SpeakUp zk-vm (mpz vm-zk). Pesapay/mpz pesa/arm-pmull = upstream SpeakUp pin + aarch64 PMULL GF(2^128).
    const DEPENDENCIES: &'static str = r#"
mpz-core = { git = "https://github.com/Pesapay/mpz", rev = "0cc76daf8d0c8b0dafb6df94111c997ee2b0cd04" }
mpz-common = { git = "https://github.com/Pesapay/mpz", rev = "0cc76daf8d0c8b0dafb6df94111c997ee2b0cd04" }
mpz-ot = { git = "https://github.com/Pesapay/mpz", rev = "0cc76daf8d0c8b0dafb6df94111c997ee2b0cd04", features = ["rayon"] }
mpz-vm-core = { git = "https://github.com/Pesapay/mpz", rev = "0cc76daf8d0c8b0dafb6df94111c997ee2b0cd04" }
mpz-vm-ir = { git = "https://github.com/Pesapay/mpz", rev = "0cc76daf8d0c8b0dafb6df94111c997ee2b0cd04" }
mpz-vm-zk = { git = "https://github.com/Pesapay/mpz", rev = "0cc76daf8d0c8b0dafb6df94111c997ee2b0cd04" }
rayon = "1"
rand = "0.9"
futures = "0.3"
tokio = { version = "1", features = ["rt-multi-thread", "net", "time"] }
tokio-util = { version = "0.7", features = ["compat"] }
# Pure-Rust blake3: no NDK clang needed, no C symbol clashes with other static libs.
blake3 = { version = "1", features = ["pure"] }
    "#;

    const DEV_DEPENDENCIES: &'static str = r#"
sha2 = "0.10"
    "#;
}

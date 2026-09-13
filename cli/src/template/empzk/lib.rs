// --- emp-zk: Wolverine/QuickSilver boolean ZK (emp-toolkit) ---

#[cfg(not(target_arch = "wasm32"))]
mod empzk;
#[cfg(not(target_arch = "wasm32"))]
pub use empzk::{empzk_prove_sha256_loopback, EmpzkBenchResult};

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod empzk_tests {
    use super::empzk_prove_sha256_loopback;

    #[test]
    fn test_empzk_sha256_loopback() {
        // verifier_accepted = both parties' revealed digest equals OpenSSL SHA-256 of the message.
        let r = empzk_prove_sha256_loopback(1024).unwrap();
        assert_eq!(
            r.digest_hex,
            "e9183d9a79aad8a047b8e67981210d50b01fc75b1edba5bc32ba3d3ec4d5056d"
        );
        assert!(r.verifier_accepted);
        assert!(empzk_prove_sha256_loopback(1000).is_err());
    }
}

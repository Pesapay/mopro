//! emp-zk (emp-toolkit): interactive, designated-verifier boolean ZK (Wolverine/QuickSilver)
//! over SilentFerret COT. Same statement as the SpeakUp benchmark: SHA-256 of an N-byte
//! private message, prover and verifier on this device over loopback TCP.

use crate::MoproError;

#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[derive(Clone, Debug)]
pub struct EmpzkBenchResult {
    pub digest_hex: String,
    pub total_ms: u64,
    pub prover_sent_bytes: u64,
    pub prover_received_bytes: u64,
    pub verifier_accepted: bool,
}

/// `len` must be 1024, 4096, 16384 or 65536. Blocking; call it off the UI thread.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn empzk_prove_sha256_loopback(len: u32) -> Result<EmpzkBenchResult, MoproError> {
    let r = emp_zk_mopro::sha256_loopback(len).map_err(MoproError::EmpzkError)?;
    Ok(EmpzkBenchResult {
        digest_hex: r.digest_hex,
        total_ms: r.total_ms,
        prover_sent_bytes: r.prover_sent_bytes,
        prover_received_bytes: r.prover_received_bytes,
        verifier_accepted: r.verifier_accepted,
    })
}

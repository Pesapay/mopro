// --- SpeakUp: interactive VOLE-ZK zk-vm proving WebAssembly guests (mpz vm-zk) ---

#[cfg(not(target_arch = "wasm32"))]
mod speakup;
#[cfg(not(target_arch = "wasm32"))]
pub use speakup::{
    speakup_builtin_guest, speakup_init_threads, speakup_prove_sha256_loopback,
    speakup_prove_sha256_remote, speakup_prover_connect, speakup_verifier_accept,
    speakup_verify_sha256_once, SpeakupArg, SpeakupBenchResult, SpeakupParty, SpeakupWireStats,
};

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod speakup_tests {
    use super::*;
    use futures::{executor::block_on, future::join};
    use sha2::{Digest, Sha256};

    fn public(v: i64) -> SpeakupArg {
        SpeakupArg { kind: "public".into(), ty: "i32".into(), value: v }
    }

    #[test]
    fn test_speakup_sha256_loopback() {
        let r = speakup_prove_sha256_loopback(1536, None).unwrap();
        let msg: Vec<u8> = (0..1536u32).map(|i| (i.wrapping_mul(7).wrapping_add(3)) as u8).collect();
        let want: String = Sha256::digest(&msg).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(r.digest_hex, want);
        assert!(r.verifier_accepted);
        assert!(r.prover_sent_bytes > 0 && r.prover_received_bytes > 0);
    }

    #[test]
    fn test_speakup_sudoku_accepts_valid_rejects_wrong() {
        let puzzle: Vec<u8> = "530070000600195000098000060800060003400803001700020006060000280000419005000080079"
            .bytes().map(|c| c - b'0').collect();
        let solution: Vec<u8> = "534678912672195348198342567859761423426853791713924856961537284287419635345286179"
            .bytes().map(|c| c - b'0').collect();
        let mut wrong = solution.clone();
        wrong.swap(2, 3);
        let wasm = speakup_builtin_guest("sudoku".into()).unwrap();
        for (port, sol, want) in [(47301, solution, 1), (47302, wrong, 0)] {
            let addr = format!("127.0.0.1:{port}");
            let (v, p) = block_on(join(
                speakup_verifier_accept(addr.clone(), wasm.clone(), None),
                speakup_prover_connect(addr, wasm.clone(), None),
            ));
            let run = |party: std::sync::Arc<SpeakupParty>, sol: Option<Vec<u8>>| {
                let puzzle = puzzle.clone();
                async move {
                    let alloc = |n| party.call_local("cabi_realloc".into(), vec![public(0), public(0), public(1), public(n)]);
                    let pz = alloc(81)?.unwrap() as u32;
                    party.write_public(pz, puzzle)?;
                    let sp = alloc(81)?.unwrap() as u32;
                    match sol {
                        Some(s) => party.write_private(sp, s)?,
                        None => party.write_blind(sp, 81)?,
                    }
                    party.call("check".into(), vec![public(pz as i64), public(sp as i64)]).await
                }
            };
            let (vr, pr) = block_on(join(run(v.unwrap(), None), run(p.unwrap(), Some(sol))));
            assert_eq!(vr.unwrap(), Some(want));
            assert_eq!(pr.unwrap(), Some(want));
        }
    }

    #[test]
    fn test_speakup_length_mismatch_rejected() {
        let addr = "127.0.0.1:47303".to_string();
        let verifier = std::thread::spawn({
            let addr = addr.clone();
            move || speakup_verify_sha256_once(addr, 1536, None)
        });
        let prover = speakup_prove_sha256_remote(addr, 1535, None);
        assert!(verifier.join().unwrap().is_err());
        assert!(prover.is_err());
    }
}

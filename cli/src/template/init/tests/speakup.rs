mopro_ffi::uniffi_setup!();

#[cfg(target_os = "macos")]
uniffi::build_foreign_language_testcases!("tests/bindings/speakup/test_speakup_sha256.swift",);

uniffi::build_foreign_language_testcases!("tests/bindings/speakup/test_speakup_sha256.kts",);

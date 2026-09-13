// GENERATED LIB IMPORT PLACEHOLDER

try {
    val result = speakupProveSha256Loopback(1024u, null)
    assert(result.verifierAccepted) { "Verifier should accept the proof" }
    assert(result.digestHex.length == 64) { "Digest should be 32 bytes of hex" }
    assert(result.proverSentBytes > 0uL) { "Prover should have sent protocol traffic" }
} catch (e: Exception) {
    println(e)
    throw e
}

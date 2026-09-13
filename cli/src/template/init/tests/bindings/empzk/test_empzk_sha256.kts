// GENERATED LIB IMPORT PLACEHOLDER

try {
    val result = empzkProveSha256Loopback(1024u)
    assert(result.verifierAccepted) { "Verifier should accept the proof" }
    assert(result.digestHex.length == 64) { "Digest should be 32 bytes of hex" }
} catch (e: Exception) {
    println(e)
    throw e
}

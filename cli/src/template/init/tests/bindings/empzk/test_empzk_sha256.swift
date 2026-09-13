import Foundation
// GENERATED LIB IMPORT PLACEHOLDER

do {
    let result = try empzkProveSha256Loopback(len: 1024)
    assert(result.verifierAccepted, "Verifier should accept the proof")
    assert(result.digestHex.count == 64, "Digest should be 32 bytes of hex")
} catch let error as MoproError {
    print("MoproError: \(error)")
    throw error
} catch {
    print("Unexpected error: \(error)")
    throw error
}

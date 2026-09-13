//
//  ContentView.swift
//  MoproApp
//
import SwiftUI

struct ContentView: View {
  @State private var textViewText = ""
  @State private var isCircomProveButtonEnabled = true
  @State private var isCircomVerifyButtonEnabled = false
  @State private var isRapidsnarkProveButtonEnabled = true
  @State private var isRapidsnarkVerifyButtonEnabled = false
  @State private var isGnarkProveButtonEnabled = true
  @State private var isGnarkVerifyButtonEnabled = false
  @State private var isHalo2ProveButtonEnabled = true
  @State private var isHalo2VerifyButtonEnabled = false
  @State private var isNoirProveButtonEnabled = true
  @State private var isNoirVerifyButtonEnabled = false
  @State private var isSpeakupProveButtonEnabled = true
  @State private var isEmpzkProveButtonEnabled = true
  @State private var generatedCircomProof: CircomProof?
  @State private var circomPublicInputs: [String]?
  @State private var generatedRapidsnarkProof: CircomProof?
  @State private var rapidsnarkPublicInputs: [String]?
  @State private var generatedGnarkProof: String?
  @State private var gnarkPublicInputs: String?
  @State private var generatedHalo2Proof: Data?
  @State private var halo2PublicInputs: Data?
  @State private var generatedNoirProof: Data?
  @State private var noirVerificationKey: Data?
  private let zkeyPath = Bundle.main.path(forResource: "multiplier2_final", ofType: "zkey")!
  private let witnesscalc_zkeyPath = Bundle.main.path(
    forResource: "multiplier2_wc_final", ofType: "zkey")!
  private let srsPath = Bundle.main.path(forResource: "plonk_fibonacci_srs.bin", ofType: "")!
  private let vkPath = Bundle.main.path(forResource: "plonk_fibonacci_vk.bin", ofType: "")!
  private let pkPath = Bundle.main.path(forResource: "plonk_fibonacci_pk.bin", ofType: "")!
  private let noirSrsPath = Bundle.main.path(forResource: "noir_multiplier2", ofType: "srs")!
  private let noirCircuitPath = Bundle.main.path(forResource: "noir_multiplier2", ofType: "json")!
  private let noirVkPath = Bundle.main.path(forResource: "noir_multiplier2", ofType: "vk")!
  private let gnarkR1csPath = Bundle.main.path(forResource: "cubic_circuit", ofType: "r1cs")!
  private let gnarkPkPath = Bundle.main.path(forResource: "cubic_circuit", ofType: "pk")!
  private let gnarkVkPath = Bundle.main.path(forResource: "cubic_circuit", ofType: "vk")!

  var body: some View {
    VStack(spacing: 10) {
      Image(systemName: "globe")
        .imageScale(.large)
        .foregroundStyle(.tint)
      Button("Prove Circom", action: runCircomProveAction).disabled(!isCircomProveButtonEnabled)
        .accessibilityIdentifier("proveCircom")
      Button("Verify Circom", action: runCircomVerifyAction).disabled(!isCircomVerifyButtonEnabled)
        .accessibilityIdentifier("verifyCircom")
      Button("Prove Circom (Rapidsnark)", action: runRapidsnarkProveAction).disabled(
        !isRapidsnarkProveButtonEnabled
      ).accessibilityIdentifier("proveRapidsnark")
      Button("Verify Circom (Rapidsnark)", action: runRapidsnarkVerifyAction).disabled(
        !isRapidsnarkVerifyButtonEnabled
      ).accessibilityIdentifier("verifyRapidsnark")
      Button("Prove Gnark", action: runGnarkProveAction).disabled(!isGnarkProveButtonEnabled)
        .accessibilityIdentifier("proveGnark")
      Button("Verify Gnark", action: runGnarkVerifyAction).disabled(!isGnarkVerifyButtonEnabled)
        .accessibilityIdentifier("verifyGnark")
      Button("Prove Halo2", action: runHalo2ProveAction).disabled(!isHalo2ProveButtonEnabled)
        .accessibilityIdentifier("proveHalo2")
      Button("Verify Halo2", action: runHalo2VerifyAction).disabled(!isHalo2VerifyButtonEnabled)
        .accessibilityIdentifier("verifyHalo2")
      Button("Prove Noir", action: runNoirProveAction).disabled(!isNoirProveButtonEnabled)
        .accessibilityIdentifier("proveNoir")
      Button("Verify Noir", action: runNoirVerifyAction).disabled(!isNoirVerifyButtonEnabled)
        .accessibilityIdentifier("verifyNoir")
      Button("Prove SpeakUp (sha256)", action: runSpeakupProveAction).disabled(
        !isSpeakupProveButtonEnabled
      ).accessibilityIdentifier("proveSpeakup")
      Button("Prove emp-zk (sha256)", action: runEmpzkProveAction).disabled(
        !isEmpzkProveButtonEnabled
      ).accessibilityIdentifier("proveEmpzk")

      ScrollView {
        Text(textViewText)
          .padding()
          .accessibilityIdentifier("proof_log")
      }
      .frame(height: 200)
    }
    .padding()
    .onAppear {
      let args = ProcessInfo.processInfo.arguments
      if args.contains("-speakup-bench") { runSpeakupProveAction() }
      if args.contains("-empzk-bench") { runEmpzkProveAction() }
      if args.contains("-compare-bench") { runCompareBenchAction() }
    }
  }
}

extension ContentView {
  func runCircomProveAction() {
    textViewText += "Generating Circom proof... "
    do {
      // Prepare inputs
      let a = 3
      let b = 5
      let c = a * b
      let input_str: String = "{\"b\":[\"5\"],\"a\":[\"3\"]}"

      // Expected outputs
      let outputs: [String] = [String(c), String(a)]

      let start = CFAbsoluteTimeGetCurrent()

      // Generate Proof
      let generateProofResult = try generateCircomProof(
        zkeyPath: zkeyPath, circuitInputs: input_str, proofLib: ProofLib.arkworks)
      assert(!generateProofResult.proof.a.x.isEmpty, "Proof should not be empty")
      assert(outputs == generateProofResult.inputs, "Circuit outputs mismatch the expected outputs")

      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      // Store the generated proof and public inputs for later verification
      generatedCircomProof = generateProofResult.proof
      circomPublicInputs = generateProofResult.inputs

      textViewText += "\(String(format: "%.3f", timeTaken))s 1️⃣\n"

      isCircomVerifyButtonEnabled = true
    } catch {
      textViewText += "\nProof generation failed: \(error.localizedDescription)\n"
    }
  }

  func runCircomVerifyAction() {
    guard let proof = generatedCircomProof,
      let inputs = circomPublicInputs
    else {
      textViewText += "Proof has not been generated yet.\n"
      return
    }

    textViewText += "Verifying Circom proof... "
    do {
      let start = CFAbsoluteTimeGetCurrent()

      let isValid = try verifyCircomProof(
        zkeyPath: zkeyPath, proofResult: CircomProofResult(proof: proof, inputs: inputs),
        proofLib: ProofLib.arkworks)
      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      assert(proof.a.x.count > 0, "Proof should not be empty")
      assert(inputs.count > 0, "Inputs should not be empty")

      print("Ethereum Proof: \(proof)\n")
      print("Ethereum Inputs: \(inputs)\n")

      if isValid {
        textViewText += "\(String(format: "%.3f", timeTaken))s 2️⃣\n"
      } else {
        textViewText += "\nProof verification failed.\n"
      }
      isCircomVerifyButtonEnabled = false
    } catch let error as MoproError {
      print("\nMoproError: \(error)")
    } catch {
      print("\nUnexpected error: \(error)")
    }
  }

  func runRapidsnarkProveAction() {
    textViewText += "Generating Circom Rapidsnark proof... "
    do {
      // Prepare inputs
      let a = 3
      let b = 5
      let c = a * b
      let input_str: String = "{\"b\":[\"5\"],\"a\":[\"3\"]}"

      // Expected outputs
      let outputs: [String] = [String(c), String(a)]

      let start = CFAbsoluteTimeGetCurrent()

      // Generate Proof
      let generateProofResult = try generateCircomProof(
        zkeyPath: witnesscalc_zkeyPath, circuitInputs: input_str, proofLib: ProofLib.rapidsnark)
      assert(!generateProofResult.proof.a.x.isEmpty, "Proof should not be empty")
      assert(outputs == generateProofResult.inputs, "Circuit outputs mismatch the expected outputs")

      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      // Store the generated proof and public inputs for later verification
      generatedCircomProof = generateProofResult.proof
      circomPublicInputs = generateProofResult.inputs

      textViewText += "\(String(format: "%.3f", timeTaken))s 1️⃣\n"

      isRapidsnarkVerifyButtonEnabled = true
    } catch {
      textViewText += "\nProof generation failed: \(error.localizedDescription)\n"
    }
  }

  func runRapidsnarkVerifyAction() {
    guard let proof = generatedCircomProof,
      let inputs = circomPublicInputs
    else {
      textViewText += "Proof has not been generated yet.\n"
      return
    }

    textViewText += "Verifying Circom Rapidsnark proof... "
    do {
      let start = CFAbsoluteTimeGetCurrent()

      let isValid = try verifyCircomProof(
        zkeyPath: witnesscalc_zkeyPath,
        proofResult: CircomProofResult(proof: proof, inputs: inputs), proofLib: ProofLib.rapidsnark)
      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      assert(proof.a.x.count > 0, "Proof should not be empty")
      assert(inputs.count > 0, "Inputs should not be empty")

      print("Ethereum Proof: \(proof)\n")
      print("Ethereum Inputs: \(inputs)\n")

      if isValid {
        textViewText += "\(String(format: "%.3f", timeTaken))s 2️⃣\n"
      } else {
        textViewText += "\nProof verification failed.\n"
      }
      isCircomVerifyButtonEnabled = false
    } catch let error as MoproError {
      print("\nMoproError: \(error)")
    } catch {
      print("\nUnexpected error: \(error)")
    }
  }

  func runGnarkProveAction() {
    textViewText += "Generating Gnark proof... "
    do {
      // Prepare inputs
      let witnessJson = "{\"X\": \"3\", \"Y\": \"35\"}"

      let start = CFAbsoluteTimeGetCurrent()

      // Generate Proof
      let generateProofResult = try generateGnarkProof(
        r1csPath: gnarkR1csPath, pkPath: gnarkPkPath, witnessJson: witnessJson)
      assert(!generateProofResult.proof.isEmpty, "Proof should not be empty")
      assert(!generateProofResult.publicInputs.isEmpty, "Public inputs should not be empty")

      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      // Store the generated proof and public inputs for later verification
      generatedGnarkProof = generateProofResult.proof
      gnarkPublicInputs = generateProofResult.publicInputs

      textViewText += "\(String(format: "%.3f", timeTaken))s 1️⃣\n"

      isGnarkVerifyButtonEnabled = true
    } catch {
      textViewText += "\nProof generation failed: \(error.localizedDescription)\n"
    }
  }

  func runGnarkVerifyAction() {
    guard let proof = generatedGnarkProof,
      let publicInputs = gnarkPublicInputs
    else {
      textViewText += "Proof has not been generated yet.\n"
      return
    }

    textViewText += "Verifying Gnark proof... "
    do {
      let start = CFAbsoluteTimeGetCurrent()

      let isValid = try verifyGnarkProof(
        r1csPath: gnarkR1csPath, vkPath: gnarkVkPath,
        proofResult: GnarkProofResult(proof: proof, publicInputs: publicInputs))
      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      assert(isValid, "Proof verification should succeed")

      if isValid {
        textViewText += "\(String(format: "%.3f", timeTaken))s 2️⃣\n"
      } else {
        textViewText += "\nProof verification failed.\n"
      }
      isGnarkVerifyButtonEnabled = false
    } catch let error as MoproError {
      print("\nMoproError: \(error)")
    } catch {
      print("\nUnexpected error: \(error)")
    }
  }

  func runHalo2ProveAction() {
    textViewText += "Generating Halo2 proof... "
    do {
      // Prepare inputs
      var inputs = [String: [String]]()
      let out = 55
      inputs["out"] = [String(out)]

      let start = CFAbsoluteTimeGetCurrent()

      // Generate Proof
      let generateProofResult = try generateHalo2Proof(
        srsPath: srsPath, pkPath: pkPath, circuitInputs: inputs)
      assert(!generateProofResult.proof.isEmpty, "Proof should not be empty")
      assert(!generateProofResult.inputs.isEmpty, "Inputs should not be empty")

      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      // Store the generated proof and public inputs for later verification
      generatedHalo2Proof = generateProofResult.proof
      halo2PublicInputs = generateProofResult.inputs

      textViewText += "\(String(format: "%.3f", timeTaken))s 1️⃣\n"

      isHalo2VerifyButtonEnabled = true
    } catch {
      textViewText += "\nProof generation failed: \(error.localizedDescription)\n"
    }
  }

  func runHalo2VerifyAction() {
    guard let proof = generatedHalo2Proof,
      let inputs = halo2PublicInputs
    else {
      textViewText += "Proof has not been generated yet.\n"
      return
    }

    textViewText += "Verifying Halo2 proof... "
    do {
      let start = CFAbsoluteTimeGetCurrent()

      let isValid = try verifyHalo2Proof(
        srsPath: srsPath, vkPath: vkPath, proof: proof, publicInput: inputs)
      let end = CFAbsoluteTimeGetCurrent()
      let timeTaken = end - start

      if isValid {
        textViewText += "\(String(format: "%.3f", timeTaken))s 2️⃣\n"
      } else {
        textViewText += "\nProof verification failed.\n"
      }
      isHalo2VerifyButtonEnabled = false
    } catch let error as MoproError {
      print("\nMoproError: \(error)")
    } catch {
      print("\nUnexpected error: \(error)")
    }
  }

  func runNoirProveAction() {
    textViewText += "Generating Noir proof...\n"

    do {
      let inputs: [String] = ["3", "5"]
      let onChain = true  // Use Keccak for Solidity compatibility
      let lowMemoryMode = false

      DispatchQueue.global(qos: .userInitiated).async {
        do {
          // First, try to load existing verification key from file, or generate new one
          let vk: Data
          if let existingVkData = try? Data(contentsOf: URL(fileURLWithPath: noirVkPath)) {
            vk = existingVkData
            DispatchQueue.main.async {
              textViewText += "Using existing verification key...\n"
            }
          } else {
            DispatchQueue.main.async {
              textViewText += "Generating verification key...\n"
            }
            vk = try getNoirVerificationKey(
              circuitPath: noirCircuitPath, srsPath: noirSrsPath, onChain: onChain,
              lowMemoryMode: lowMemoryMode)
          }
          noirVerificationKey = vk

          DispatchQueue.main.async {
            textViewText += "Generating proof with verification key...\n"
          }
          let start = CFAbsoluteTimeGetCurrent()

          // Generate the proof with all required parameters
          let proofData = try generateNoirProof(
            circuitPath: noirCircuitPath,
            srsPath: noirSrsPath,
            inputs: inputs,
            onChain: onChain,
            vk: vk,
            lowMemoryMode: lowMemoryMode
          )

          let end = CFAbsoluteTimeGetCurrent()
          let timeTaken = end - start

          DispatchQueue.main.async {
            generatedNoirProof = proofData
            textViewText += "\(String(format: "%.3f", timeTaken))s 1️⃣\n"
            isNoirVerifyButtonEnabled = true
          }
        } catch {
          DispatchQueue.main.async {
            textViewText += "Proof generation failed: \(error.localizedDescription)\n"
          }
        }
      }

    } catch {
      textViewText += "Error setting up proof generation: \(error.localizedDescription)\n"
    }
  }

  func runNoirVerifyAction() {
    guard let proofData = generatedNoirProof else {
      textViewText += "Error: Proof data is not available. Generate proof first.\n"
      return
    }

    guard let vk = noirVerificationKey else {
      textViewText += "Error: Verification key is not available. Generate proof first.\n"
      return
    }

    textViewText += "Verifying Noir proof...\n"

    DispatchQueue.global(qos: .userInitiated).async {
      let start = CFAbsoluteTimeGetCurrent()
      do {
        let onChain = true  // Use Keccak for Solidity compatibility
        let lowMemoryMode = false

        // Verify the proof with all required parameters
        let isValid = try verifyNoirProof(
          circuitPath: noirCircuitPath,
          proof: proofData,
          onChain: onChain,
          vk: vk,
          lowMemoryMode: lowMemoryMode
        )

        let end = CFAbsoluteTimeGetCurrent()
        let timeTaken = end - start

        DispatchQueue.main.async {
          if isValid {
            textViewText += "\(String(format: "%.3f", timeTaken))s 2️⃣\n"
          } else {
            textViewText += "\nProof verification failed.\n"
          }
          isNoirVerifyButtonEnabled = false
        }
      } catch {
        DispatchQueue.main.async {
          textViewText += "Verification failed: \(error.localizedDescription)\n"
        }
      }
    }
  }
}

extension ContentView {
  // Prover and verifier both on-device over loopback; median of 3 after a warm-up.
  func runSpeakupProveAction() {
    textViewText += "Proving SpeakUp sha256 (loopback)...\n"
    isSpeakupProveButtonEnabled = false
    let args = ProcessInfo.processInfo.arguments
    // 2M sVOLE per chunk bounds peak memory; mpz's 15M default thrashes a phone at 64 KB.
    let cap = args.firstIndex(of: "-speakup-cap").flatMap { UInt64(args[$0 + 1]) } ?? 2_000_000
    DispatchQueue.global(qos: .userInitiated).async {
      for len: UInt32 in [1024, 4096, 16384, 65536] {
        do {
          _ = try speakupProveSha256Loopback(len: len, chunkCap: cap)
          var times: [UInt64] = []
          var last: SpeakupBenchResult?
          for _ in 0..<3 {
            let r = try speakupProveSha256Loopback(len: len, chunkCap: cap)
            times.append(r.totalMs)
            last = r
          }
          let median = times.sorted()[1]
          let line =
            "speakup sha256 \(len / 1024) KB cap \(cap): \(median) ms \(times) | P->V \(last!.proverSentBytes / 1024) KB, V->P \(last!.proverReceivedBytes / 1024) KB | accepted \(last!.verifierAccepted)"
          print(line)
          DispatchQueue.main.async { textViewText += line + "\n" }
        } catch {
          print("speakup error: \(error)")
          DispatchQueue.main.async { textViewText += "SpeakUp error: \(error)\n" }
        }
      }
      DispatchQueue.main.async { isSpeakupProveButtonEnabled = true }
    }
  }
}

extension ContentView {
  func runEmpzkProveAction() {
    textViewText += "Proving emp-zk sha256 (loopback)...\n"
    isEmpzkProveButtonEnabled = false
    DispatchQueue.global(qos: .userInitiated).async {
      for len: UInt32 in [1024, 4096, 16384, 65536] {
        do {
          _ = try empzkProveSha256Loopback(len: len)
          var times: [UInt64] = []
          var last: EmpzkBenchResult?
          for _ in 0..<3 {
            let r = try empzkProveSha256Loopback(len: len)
            times.append(r.totalMs)
            last = r
          }
          let line =
            "empzk sha256 \(len / 1024) KB: \(times.sorted()[1]) ms \(times) | P->V \(last!.proverSentBytes / 1024) KB, V->P \(last!.proverReceivedBytes / 1024) KB | accepted \(last!.verifierAccepted)"
          print(line)
          DispatchQueue.main.async { textViewText += line + "\n" }
        } catch {
          print("empzk error: \(error)")
          DispatchQueue.main.async { textViewText += "emp-zk error: \(error)\n" }
        }
      }
      DispatchQueue.main.async { isEmpzkProveButtonEnabled = true }
    }
  }

  // SpeakUp vs emp-zk on the same statement, alternating A/B/B/A per size so drift hits both.
  func runCompareBenchAction() {
    let args = ProcessInfo.processInfo.arguments
    let cap = args.firstIndex(of: "-speakup-cap").flatMap { UInt64(args[$0 + 1]) } ?? 2_000_000
    let rounds = args.firstIndex(of: "-rounds").flatMap { Int(args[$0 + 1]) } ?? 6
    DispatchQueue.global(qos: .userInitiated).async {
      for len: UInt32 in [1024, 4096, 16384, 65536] {
        do {
          _ = try speakupProveSha256Loopback(len: len, chunkCap: cap)
          _ = try empzkProveSha256Loopback(len: len)
          var sp: [UInt64] = []
          var ez: [UInt64] = []
          var spBytes = (UInt64(0), UInt64(0))
          var ezBytes = (UInt64(0), UInt64(0))
          for i in 0..<rounds {
            let order = i % 2 == 0 ? [0, 1] : [1, 0]
            for which in order {
              if which == 0 {
                let r = try speakupProveSha256Loopback(len: len, chunkCap: cap)
                precondition(r.verifierAccepted)
                sp.append(r.totalMs)
                spBytes = (r.proverSentBytes, r.proverReceivedBytes)
              } else {
                let r = try empzkProveSha256Loopback(len: len)
                precondition(r.verifierAccepted)
                ez.append(r.totalMs)
                ezBytes = (r.proverSentBytes, r.proverReceivedBytes)
              }
            }
          }
          let med = { (xs: [UInt64]) in xs.sorted()[xs.count / 2] }
          let line =
            "compare sha256 \(len / 1024) KB | speakup \(med(sp)) ms \(sp) P->V \(spBytes.0 / 1024) KB V->P \(spBytes.1 / 1024) KB | empzk \(med(ez)) ms \(ez) P->V \(ezBytes.0 / 1024) KB V->P \(ezBytes.1 / 1024) KB"
          print(line)
          DispatchQueue.main.async { textViewText += line + "\n" }
        } catch {
          print("compare error: \(error)")
        }
      }
      print("compare done")
    }
  }
}

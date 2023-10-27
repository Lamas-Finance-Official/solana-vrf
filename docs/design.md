# VRF on Solana

## Verifiable Random Function

There are a few different VRF algorithm out in the wild, but we will choose to use the rust crate [vrf-rs](https://github.com/witnet/vrf-rs) which implement the algorithms described in:

- [VRF-draft-05](https://datatracker.ietf.org/doc/pdf/draft-irtf-cfrg-vrf-05)

- [RFC6979](https://www.rfc-editor.org/rfc/rfc6979)

The main reason is that the repo is fairly active and written in Rust.

The library expose 3 main function:

```rs
prove(key: SecretKey, seeds: &[u8]) -> VrfProof

proof_to_hash(proof: &VrfProof) -> Hash

verify(key: PublicKey, prove: VrfProof, seeds: &[u8]) -> Hash
```

To generate a new random hash we will:

- Prepare a secure keypair

- Gather some unpredictable data to use as seeds

- Generate the proof using `prove(SecretKey, Seeds)`

- Finally, derive the output hash from the proof

## Sketch implementation of VRF on Solana

The flow of getting a random value on Solana blockchain will be something like:

- (on-chain) Gather some publicly known but unpredictable data to use as `seeds`.

- (on-chain) The contract has to somehow notify the VRF-server that it requires a new random value, along with necessary data to continue the process after receiving the random value. We will call these necessary values the contract's `state`.

- (off-chain) Generate the `proof` and `hash` from our `PrivateKey` and `seeds`.

- (off-chain) Supply the `hash` as the random value and `proof` along with the `state` to the contract.

- ~~(on-chain) Verify that the random value is fair using the `proof`, `seeds` and `PublicKey`.~~

- (on-chain) Using the random value to continue processing.

With this design, we will need at the very least 2 transactions (with 1 from the VRF-server itself) to have a random value to work with.

### Verify the proof on-chain

For now we will NOT verify the proof on-chain.

Instead we will store both the seeds, proof and the random result and make it public if needed.

Note: The reason is: vrf-rs depends on OpenSsl which is a c library and thus does not work on-chain

## Current implementation

- (on-chain) Use recent block hashes as random seeds

- (on-chain) Emit anchor event (just a log message contains base64 encoded data) to notify the VRF-server

- (off-chain) VRF-server listen to transaction logs and check if the contract has emit the event

- (off-chain) Generate the `proof` and `hash` from our `PrivateKey` and `seeds`.

- (off-chain) Supply the `hash` as the random value and `proof` along with the `state` to the contract.

- (on-chain) Continue processing with the resulted random value

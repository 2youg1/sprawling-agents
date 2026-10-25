# sprawling-remote-access

The remote door, and the routes that carry its bytes and decide nothing. A person who leaves the machine their city runs on can still reach that city from a tablet or a phone: see it, answer its questions and stop it. While the remote door is shut, a city answers on loopback, or on the local network with a pairing token, and nowhere else. Rust code names the crate `remote_access`; `cargo -p` takes `sprawling-remote-access`.

The crate is split into two layers, and only the second can be replaced. The first is the security core: `door` holds whether the door is open and in which epoch, the pairings still waiting, the paired devices and their sessions; `pairing` mints the one-use pairing code; `keys` holds the city's and each device's hybrid signing keys, ML-DSA-44 with Ed25519; `handshake` runs the key exchange of every connection, X25519 with ML-KEM-768, and the first handshake of a new device; `seal` wraps each frame of a session in AES-256-GCM with a counter against replay; `confirm` holds the one request that waits for the code printed on the city's console. The second layer is `route`, which only makes the loopback listener reachable from outside: a Cloudflare named tunnel by default, a command the person writes, or a scripted route for tests. The handshake authenticates the keys pinned at pairing rather than an address, and frames are encrypted end to end, so a route sees handshake messages and ciphertext and cannot pose as either side. A route does deliver the page itself, and one that rewrote the page would read what the page reads; the person who chooses the route carries that trust.

It depends on `kernel` and `child`, and `sprawling` is the one crate that depends on it; `cargo tree --workspace -i sprawling-remote-access --depth 1 -e normal` prints the current list, and the `depmap` block in [`ARCHITECTURE.md`](../../ARCHITECTURE.md) section 3 holds what each crate may name.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); `ARCHITECTURE.md` section 11, *Specifications in Lean*, describes that layout. `spec/Door.lean`, `spec/Handshake.lean` and `spec/Confirm.lean` prove the properties of the door, the pairing handshake and the console confirmation; `Spec.lean` itself is a description, whose requirements are held by the tests its section 16 lists. Section 3 says what has not yet been shown on a real tunnel. A decision is a comment that opens with `D<n>` beside what it governs. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines the words, among them *route*, *permanence* and *console confirmation*.

## Checking it

```bash
cargo nextest run -p sprawling-remote-access --all-features   # the tests, as `just test` runs them
lake build crates.remote_access.Spec                          # the SPEC's proofs; `just models` builds every crate's
cargo nextest run -p sprawling -E 'binary(remote_door)'       # the remote door end to end, over real sockets
```

The door, the pairing code, the keys, both handshakes and the seal do no I/O, and give the same bytes on Windows, macOS and Linux. `tools/fixtures/remote-handshake` holds the vectors and the one-way fixture that the browser's half of the remote door must reproduce (SPEC section 8-12). This crate's tests write them and read them back: run the tests whose names contain `vector` or `fixture` with `GOLDEN_WRITE=1` to write them again, rather than editing the files. Under that variable the files with no randomness in them are rewritten byte for byte, and the signature and the fixture only when the committed copy no longer verifies; without it every test only reads and compares.

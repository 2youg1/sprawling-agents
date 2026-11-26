# sprawling-remote-access

The remote door's pairing, device authentication and sealed sessions let a person reach the running city from a browser while the door is open. Rust code names the crate `remote_access`, and Cargo names the package `sprawling-remote-access`.

The security core decides who gets in. `door` holds whether the door is open and in which epoch, the pairings still waiting, the paired devices and the sessions they hold; `pairing` mints the one-time pairing code; `keys` holds hybrid signing keys, ML-DSA-44 with Ed25519, derived from one 32-byte seed; `handshake` runs the key exchange of every connection, X25519 with ML-KEM-768, and its `pairing` half runs a device's first one; `seal` wraps every frame of a session in AES-256-GCM with a counter against replay; `confirm` holds the one request that waits for the code the city prints on its console.

The handshake authenticates the keys pinned at pairing. Each application frame is sealed end to end. The browser also trusts the WebUI served by the city process.

This crate owns no socket, no listener, no WebSocket, no device table on disk, no clock and no entropy source. The remote listener, the relay that hands each opened frame to the city's `/ws`, the device table, the console itself and the `[remote]` configuration table all belong to the assembly layer in `Sprawling/app` (`bin::outside`, `bin::assembly::remote_door`).

## How it is shaped

Each module holds one decision:

- `door` — a state machine. The door starts closed; opening it takes a new epoch, which voids every code and session of the old one; closing it ends every session and keeps the paired devices; a session's lifetime is the sooner of what the caller asked and the moment the door closes; `authority` answers `None` once a session is gone. `permits(Authority, VerbClass)` is the one decision about which class of verb a `Watch` or `Act` device may reach.
- `pairing` — a value. `PairingCode::mint` takes 16 bytes of entropy and answers the code plus the text a person copies, shown in groups of five; `read` accepts that text however it was retyped. The door keeps a digest, never the text.
- `keys` — a value. One seed derives both halves through HKDF-SHA256 with a versioned salt, so the halves share no key material; a signature holds only when both halves hold, and a caller is never told which half failed. `written` produces the text the vault stores, and `from_sealed` reads it back; that function holds this crate's one exposure of a seed.
- `handshake` — a state machine. Three messages, `Hello`, `Reply` and `Finish`; the transcript is hashed with SHA-256 because the other end is a browser, whose WebCrypto has SHA-256 and not BLAKE3; the two direction keys are expanded from both shared secrets under that transcript. `DeviceWaiting` and `CityWaiting` are consumed by their second step, so a handshake cannot be resumed or replayed.
- `handshake::pairing` — a state machine, the device's first connection. The city alone signs the first message, so the pairing code never travels in the clear; the device checks the city's fingerprint before it answers, and the claim carrying the code arrives sealed under keys derived from that exchange.
- `seal` — a state machine. The nonce is a direction tag and a counter that is never sent, so a frame replayed, dropped, reordered or turned around fails to open, and a frame that fails ends the session. The payload's first byte says whether it is a wire frame or a lock.
- `confirm` — a state machine with a guard. One request waits at a time; every answer the guard reads ends it, so one request gives one guess. The guard refuses an answer that comes sooner than a second after the previous one, and locks after ten misses until the city restarts.

The city process serves its embedded WebUI and the remote door over HTTP/3 and WebTransport. It rereads the city configuration on each opening.

## When a call fails

A refusal is an `AxError` in three parts: the failed action, the subject, and a recovery. The codes below are the whole set this crate answers with; `Sprawling/remote_access/Spec.lean` section 12 holds the table with the state each one leaves behind.

| Code | When | What a caller does |
|---|---|---|
| `E_GATE_DENIED` | The door is shut, the code is wrong, the device is unpaired, a hybrid signature fails on one half, a fingerprint or a city signature does not check out, a claim does not open, or a sealed frame does not open | Stop the connection. A code taken from the waiting table stays used whether it was answered or not. |
| `E_INVALID_ARGS` | An epoch is reused, an unknown device is revoked, a device name is empty or past its limit, a public key or signature is the wrong length, or a written device or fingerprint does not read back | Fix the value. A device table with one unreadable row is refused whole by the assembly layer. |
| `E_WIRE_MISMATCH` | A handshake message is the wrong length, or a payload's first byte is unknown, a lock carries trailing bytes, a frame is not UTF-8, or a payload is empty | End the connection; the other end is not speaking this protocol. |
| `E_BUDGET_EXHAUSTED` | One direction's frame counter is used up | Reconnect: a new session brings new keys and a counter from zero. |
| `E_CONFIG_INVALID` | The vault’s city-key text does not read back | Correct the city key. Until it reads back, the door refuses invitations and handshakes. |
| `E_STORAGE_FATAL` | The cryptography library refuses a derivation, a signature or a seal that should have succeeded | Nothing was produced; restart the city and report the platform. |

The city assembles the HTTP/3 listener when the door opens. Invalid HTTPS or TLS settings and UDP bind failures are reported against the city configuration. See [Choosing the route](../../docs/operate/operating.md#choosing-the-route). `Confirm::request` refuses once its guard has locked.

## Constraints a reader acts on

- The crate declares no features; nothing here can be compiled out.
- The security core does no I/O, and time and entropy arrive as parameters, so every decision replays. The one exception is the ephemeral keys of both handshakes: the cryptography library draws them, because ML-KEM encapsulation takes no external entropy.
- The assembly layer owns the city configuration and public listener.
- Opening the door, replacing the city key and confirming a request wait for the code the console prints; closing the door and locking it from a device do not (D4, D5). The code never enters a frame or a page.
- The city key's seed lives in the city's vault, not in the city directory, and `keys::from_sealed` is this crate's only exposure of a seed (D23).
- There is no `unsafe` in this crate. The vectors under `Tools/fixtures/remote-handshake` are written by its tests: run the tests whose names contain `vector` or `fixture` with `GOLDEN_WRITE=1` to write them again rather than editing the files, since the deterministic files are rewritten byte for byte and the two carrying randomness only when the committed copy no longer verifies.

## Who uses it

One workspace package depends on it: `sprawling`, which is the assembly root. `cargo tree --workspace -i sprawling-remote-access -e normal --depth 1` prints the current list.

The dependency graph in [ARCHITECTURE.md](../../docs/ARCHITECTURE.md) records the boundary between this crate and the assembly layer.

## Specification

The SPEC is [`Spec.lean`](Spec.lean), with its parts under [`spec/`](spec/); section 11 of [`ARCHITECTURE.md`](../../docs/ARCHITECTURE.md), *Specifications in Lean*, describes that layout. `spec/Door.lean` proves the six properties of the door, `spec/Handshake.lean` proves three about the pairing handshake, and `spec/Confirm.lean` proves five about the console confirmation and its guard. `Spec.lean` itself is a description: section 1 names the units, section 3 says what has not been shown on a real tunnel, section 8 holds each module's interface and the decision beside it, section 12 the error table, and section 16 the tests that hold what the proofs leave open.

A decision is a comment opening with `D<n>` directly above what it governs, cited elsewhere as `remote_access D<n>`. The SPEC is written in Chinese with concept names in English, as the *Language* section of [`AGENTS.md`](../../AGENTS.md) requires; [`docs/glossary.md`](../../docs/glossary.md) defines *route*, *permanence* and *console confirmation*, and section 6 of the SPEC separates this crate's door, epoch, session and pairing code from the words the rest of the city uses for other things.

## Building and checking

```bash
cargo nextest run -p sprawling-remote-access                    # the crate's tests
lake build Sprawling.remote_access.Spec                            # this crate's Lean specification
cargo nextest run -p sprawling -E 'binary(remote_door)'          # the remote door end to end, over real sockets
cargo xtask gates spec depmap                                   # its specification and its dependency edges
```

The door, the pairing code, the keys, both handshakes and the seal give the same bytes on Windows, macOS and Linux. The app integration tests open the public listener and pair over a real WebTransport stream. `just models` builds every crate's specification, and `just test` runs the tests the whole workspace shares.

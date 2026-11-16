<script lang="ts" module>
  import type { Device } from "../../core/remote/device";
  import type { Standing } from "../settings/remote";
  import type { DoctorCustodyLifetime } from "../../wire";

  const KEEPS: readonly (DoctorCustodyLifetime | null)[] = [
    null,
    "until_reboot",
    "this_process",
    "with_passphrase",
    "across_reboots",
    "across_reboots",
    "until_reboot",
    "across_reboots",
  ];

  const FINGERPRINT = new Uint8Array(32).map((_, at) => (at * 37 + 11) % 256);
  const ID = new Uint8Array(16).map((_, at) => (at * 53 + 7) % 256);
  const SEED = "q7mf2xkd4pyw6tnb3hvc5jrsgaez7uoi2lw4dfm6xnkp3qbtyc5a";
  const AT = Date.UTC(2026, 9, 2, 6, 41, 9, 312);

  function device(ed25519: CryptoKey): Device {
    return {
      city: new Uint8Array(1344),
      fingerprint: FINGERPRINT,
      id: ID,
      key: { public: new Uint8Array(1344), ed25519, mlDsa: new Uint8Array() },
      at: AT,
    };
  }

  function standings(ed25519: CryptoKey): readonly (readonly [string, Standing])[] {
    const invitation = { code: "abcdefghijklmnopqrstuvwxyz", city: FINGERPRINT };
    return [
      ["on the city's machine", { kind: "unpaired" }],
      ["an invitation that does not read", { kind: "unreadable" }],
      ["a browser without the curves", { kind: "lacking", lack: "crypto" }],
      ["invited", { kind: "invited", invitation, told: null, busy: false }],
      [
        "invited · answered with another city's key",
        {
          kind: "invited",
          invitation,
          told: { kind: "refusal", refusal: { kind: "refused", why: "fingerprint" } },
          busy: false,
        },
      ],
      ["the seed, once", { kind: "seed", seed: SEED, device: device(ed25519), persisted: false }],
      [
        "paired · the door locked",
        { kind: "paired", device: device(ed25519), told: { kind: "locked" }, busy: false },
      ],
      [
        "paired · the door refused",
        {
          kind: "paired",
          device: device(ed25519),
          told: { kind: "refusal", refusal: { kind: "closed", code: "E_GATE_DENIED" } },
          busy: false,
        },
      ],
    ];
  }

  const KEY = crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
</script>

<script lang="ts">
  import RemoteDoor from "../settings/remote_door.svelte";
  import RemoteState from "../settings/remote_state.svelte";
  import Case from "./case.svelte";

  const WIDTHS = [1040, 390] as const;
</script>

{#await KEY then pair}
  {#each standings(pair.privateKey) as [name, standing], at (name)}
    {#each WIDTHS as width (width)}
      <Case label={`remote group · ${name} at ${String(width)}`} {width}>
        <div class="@container/page">
          <RemoteState
            {standing}
            keeps={KEEPS[at] ?? null}
            onPair={() => undefined}
            onSeen={() => undefined}
            onLock={() => undefined}
            onForget={() => undefined}
          />
        </div>
      </Case>
    {/each}
  {/each}
{/await}
{#each WIDTHS as width (width)}
  <Case label={`remote group · the door controls at ${String(width)}`} {width}>
    <div class="@container/page">
      <RemoteDoor />
    </div>
  </Case>
{/each}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, For, onSettled } from "solid-js";
import type { Device } from "../../core/remote/device";
import type { Standing } from "../settings/remote";
import type { DoctorCustodyLifetime } from "../../wire";
import RemoteDoor from "../settings/remote_door_view";
import RemoteState from "../settings/remote_state_view";
import { GalleryCase as Case } from "./case";

export default function R3() {
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

  const [key, setKey] = createSignal<CryptoKey>();
  onSettled(() => {
    void crypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]).then((pair) => {
      setKey(pair.privateKey);
    });
  });
  const sample = () => {
    const privateKey = key();
    return privateKey === undefined ? [] : standings(privateKey);
  };
  const WIDTHS = [1040, 390] as const;
  return (
    <>
      <For each={sample()} keyed={true}>
        {([name, standing], index) => (
          <For each={WIDTHS} keyed={true}>
            {(width) => (
              <Case label={`remote group · ${name} at ${String(width)}`} width={width}>
                <div class="@container/page">
                  <RemoteState
                    standing={standing}
                    keeps={KEEPS[index()] ?? null}
                    onPair={() => undefined}
                    onSeen={() => undefined}
                    onLock={() => undefined}
                    onForget={() => undefined}
                  />
                </div>
              </Case>
            )}
          </For>
        )}
      </For>
      <For each={WIDTHS} keyed={true}>
        {(width) => (
          <Case label={`remote group · the door controls at ${String(width)}`} width={width}>
            <div class="@container/page">
              <RemoteDoor />
            </div>
          </Case>
        )}
      </For>
    </>
  );
}

<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The remote group (client-SPEC 4-57): the device's half of the remote
  // door. The door's own verbs live at the city's console and nowhere
  // else (`crates/remote_access/Spec.lean` D4), so this group pairs the
  // browser it runs in when the address bar carries an invitation, locks
  // the door behind it, and forgets it; on the city's own machine it
  // says what the door is.
  //
  // The seed is drawn once the pairing has landed, never before: a seed
  // shown for a pairing that then failed would be a key nobody pinned.
  // It lives in this component's state until the person says it is
  // written down, and is stored nowhere.

  import { onMount } from "svelte";

  import { encode } from "../../core/remote/base32";
  import { lockOver, pairOver, remoteUrl } from "../../core/remote/connect";
  import { forget, keep, kept, type Device } from "../../core/remote/device";
  import { invitationIn, isInvitation, type Invitation } from "../../core/remote/invitation";
  import { SEED_BYTES, keyFrom } from "../../core/remote/keys";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Lack, Standing } from "./remote";
  import RemoteState from "./remote_state.svelte";

  const u = ui();
  let standing = $state<Standing>({ kind: "reading" });

  onMount(() => {
    void read();
  });

  async function read(): Promise<void> {
    if (isInvitation(u.bar.hash)) {
      const invitation = invitationIn(u.bar.hash);
      const lack = lacking();
      if (invitation === null) standing = { kind: "unreadable" };
      else standing = lack === null ? { kind: "invited", invitation, told: null, busy: false } : { kind: "lacking", lack };
      return;
    }
    const device = await kept();
    standing = device === null ? { kind: "unpaired" } : { kind: "paired", device, told: null, busy: false };
  }

  // What this page cannot do without, before a key is made: an https://
  // origin, which is also the only place the door's paths are, and an
  // IndexedDB to keep the key in. WebCrypto's curves are found out when
  // the key is made.
  function lacking(): Lack | null {
    if (!window.isSecureContext || remoteUrl(window.location, "/remote/pair") === null) return "insecure";
    return typeof indexedDB === "undefined" ? "storage" : null;
  }

  async function pair(invitation: Invitation): Promise<void> {
    const url = remoteUrl(window.location, "/remote/pair");
    if (url === null) {
      standing = { kind: "lacking", lack: "insecure" };
      return;
    }
    standing = { kind: "invited", invitation, told: null, busy: true };
    const seed = crypto.getRandomValues(new Uint8Array(SEED_BYTES));
    const key = await keyFrom(seed);
    if (key === null) {
      standing = { kind: "lacking", lack: "crypto" };
      return;
    }
    const paired = await pairOver(url, invitation, key);
    if (paired.kind !== "paired") {
      standing = { kind: "invited", invitation, told: { kind: "refusal", refusal: paired }, busy: false };
      return;
    }
    const device: Device = { city: paired.city, fingerprint: invitation.city, id: paired.id, key, at: u.now() };
    if (!(await keep(device))) {
      standing = { kind: "invited", invitation, told: { kind: "unkept" }, busy: false };
      return;
    }
    const persisted = "storage" in navigator ? await navigator.storage.persist().catch(() => false) : false;
    // The code is spent; it leaves the address bar and the history entry.
    history.replaceState(null, "", toFragment({ kind: "setup", group: "remote" }));
    standing = { kind: "seed", seed: encode(seed), device, persisted };
  }

  async function lock(device: Device): Promise<void> {
    const url = remoteUrl(window.location, "/remote/session");
    if (url === null) {
      standing = { kind: "lacking", lack: "insecure" };
      return;
    }
    standing = { kind: "paired", device, told: null, busy: true };
    const locked = await lockOver(url, device);
    const told = locked.kind === "locked" ? { kind: "locked" as const } : { kind: "refusal" as const, refusal: locked };
    standing = { kind: "paired", device, told, busy: false };
  }
</script>

<RemoteState
  {standing}
  onPair={() => {
    if (standing.kind === "invited" && !standing.busy) void pair(standing.invitation);
  }}
  onSeen={() => {
    if (standing.kind === "seed") standing = { kind: "paired", device: standing.device, told: null, busy: false };
  }}
  onLock={() => {
    if (standing.kind === "paired" && !standing.busy) void lock(standing.device);
  }}
  onForget={() => {
    void forget().then(() => {
      standing = { kind: "unpaired" };
    });
  }}
/>

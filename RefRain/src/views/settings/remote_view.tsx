// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, onSettled, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";

import { lockOver, pairOver, remoteUrl } from "../../core/remote/connect";
import { forget, keep, kept, type Device } from "../../core/remote/device";
import { invitationIn, isInvitation, type Invitation } from "../../core/remote/invitation";
import { SEED_BYTES, keyFrom } from "../../core/remote/keys";
import { QUERIES } from "../../core/asking";
import { toFragment } from "../../core/route";
import { ui } from "../../ui";
import type { Lack, Standing } from "./remote";
import RemoteDoor from "./remote_door_view";
import RemoteState from "./remote_state_view";

export default function Remote() {
  const u = ui();

  const doctor = storeValue(u.conn.asking.ask(QUERIES.doctor));
  const keeps = () => {
    const now = doctor();
    return now !== undefined && "doctor" in now ? now.doctor.custody.keeps : null;
  };
  const [standing, setStanding] = createSignal<Standing>({ kind: "reading" });

  onSettled(() => {
    void read();
  });

  async function read(): Promise<void> {
    if (isInvitation(u.bar.hash)) {
      const invitation = invitationIn(u.bar.hash);
      const lack = lacking();
      if (invitation === null) setStanding({ kind: "unreadable" });
      else
        setStanding(
          lack === null
            ? { kind: "invited", invitation, told: null, busy: false }
            : { kind: "lacking", lack },
        );
      return;
    }
    const device = await kept();
    setStanding(
      device === null ? { kind: "unpaired" } : { kind: "paired", device, told: null, busy: false },
    );
  }

  function lacking(): Lack | null {
    if (!window.isSecureContext || remoteUrl(window.location, "/remote/pair") === null)
      return "insecure";
    return typeof indexedDB === "undefined" ? "storage" : null;
  }

  async function pair(invitation: Invitation): Promise<void> {
    const url = remoteUrl(window.location, "/remote/pair");
    if (url === null) {
      setStanding({ kind: "lacking", lack: "insecure" });
      return;
    }
    setStanding({ kind: "invited", invitation, told: null, busy: true });
    const seed = crypto.getRandomValues(new Uint8Array(SEED_BYTES));
    const key = await keyFrom(seed);
    seed.fill(0);
    if (key === null) {
      setStanding({ kind: "lacking", lack: "crypto" });
      return;
    }
    const paired = await pairOver(url, invitation, key);
    if (paired.kind !== "paired") {
      setStanding({
        kind: "invited",
        invitation,
        told: { kind: "refusal", refusal: paired },
        busy: false,
      });
      return;
    }
    const device: Device = {
      city: paired.city,
      fingerprint: invitation.city,
      id: paired.id,
      key,
      at: u.now(),
    };
    if (!(await keep(device))) {
      setStanding({ kind: "invited", invitation, told: { kind: "unkept" }, busy: false });
      return;
    }
    const persisted =
      "storage" in navigator ? await navigator.storage.persist().catch(() => false) : false;

    history.replaceState(null, "", toFragment({ kind: "setup", group: "remote" }));
    setStanding({
      kind: "paired",
      device,
      told: persisted ? null : { kind: "unpersisted" },
      busy: false,
    });
    u.conn.retry();
  }

  async function lock(device: Device): Promise<void> {
    const url = remoteUrl(window.location, "/remote/session");
    if (url === null) {
      setStanding({ kind: "lacking", lack: "insecure" });
      return;
    }
    setStanding({ kind: "paired", device, told: null, busy: true });
    const locked = await lockOver(url, device);
    const told =
      locked.kind === "locked"
        ? { kind: "locked" as const }
        : { kind: "refusal" as const, refusal: locked };
    setStanding({ kind: "paired", device, told, busy: false });
  }
  function pairNow(): void {
    const current = standing();
    if (current.kind === "invited" && !current.busy) void pair(current.invitation);
  }
  function lockNow(): void {
    const current = standing();
    if (current.kind === "paired" && !current.busy) void lock(current.device);
  }
  function forgetNow(): void {
    void forget().then(() => {
      setStanding({ kind: "unpaired" });
    });
  }
  const offerDoor = () => ["unpaired", "unreadable", "lacking", "paired"].includes(standing().kind);
  return (
    <div class="flex flex-col gap-wide">
      <RemoteState
        standing={standing()}
        keeps={keeps()}
        onPair={pairNow}
        onLock={lockNow}
        onForget={forgetNow}
      />
      <Show when={offerDoor()}>
        <RemoteDoor />
      </Show>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { claim, finish, pairHello, sessionHello, type Refused, type Session } from "./handshake";
import type { Invitation } from "./invitation";
import type { DeviceKey } from "./keys";
import { payloadBytes } from "./seal";

export type Path = "/remote/pair" | "/remote/session";

export type Refusal =
  | { readonly kind: "refused"; readonly why: Refused }
  | { readonly kind: "closed"; readonly code: string | null }
  | { readonly kind: "unreachable" };

export interface Paired {
  readonly kind: "paired";

  readonly city: Uint8Array<ArrayBuffer>;

  readonly id: Uint8Array<ArrayBuffer>;
}

export interface Locked {
  readonly kind: "locked";
}

export interface Opened {
  readonly kind: "opened";
  readonly door: Door;
  readonly session: Session;
}

export interface Pinned {
  readonly city: Uint8Array<ArrayBuffer>;
  readonly id: Uint8Array<ArrayBuffer>;
  readonly key: DeviceKey;
}

const NONCE_BYTES = 32;
const DEVICE_ID_BYTES = 16;

export function remoteUrl(
  location: Pick<Location, "protocol" | "host">,
  path: Path,
): string | null {
  return location.protocol === "https:" ? `wss://${location.host}${path}` : null;
}

export async function pairOver(
  url: string,
  invitation: Invitation,
  key: DeviceKey,
): Promise<Paired | Refusal> {
  const pairing = await pairHello(fresh());
  if (pairing === null) return { kind: "refused", why: "agreement" };
  const door = await reach(url);
  if (door === null) return { kind: "unreachable" };
  door.send(pairing.hello);
  const reply = await door.next();
  if (reply === null) return door.ended();
  const claimed = await claim(pairing, reply, invitation, key);
  if (typeof claimed === "string") return door.refuse(claimed);
  door.send(claimed.sealed);
  const sealedId = await door.next();
  if (sealedId === null) return door.ended();
  const id = await claimed.session.opener.open(sealedId);
  door.close();
  return id?.length !== DEVICE_ID_BYTES
    ? { kind: "refused", why: "length" }
    : { kind: "paired", city: claimed.city, id };
}

export async function sessionOver(url: string, device: Pinned): Promise<Opened | Refusal> {
  const waiting = await sessionHello(device.id, fresh());
  if (waiting === null) return { kind: "refused", why: "agreement" };
  const door = await reach(url);
  if (door === null) return { kind: "unreachable" };
  door.send(waiting.hello);
  const reply = await door.next();
  if (reply === null) return door.ended();
  const finished = await finish(waiting, reply, device.city, device.key);
  if (typeof finished === "string") return door.refuse(finished);
  door.send(finished.finish);
  return { kind: "opened", door, session: finished.session };
}

export async function lockOver(url: string, device: Pinned): Promise<Locked | Refusal> {
  const opened = await sessionOver(url, device);
  if (opened.kind !== "opened") return opened;
  const { door, session } = opened;
  const lock = await session.sealer.seal(payloadBytes({ kind: "lock" }));
  if (lock === null) return door.refuse("key");
  door.send(lock);
  const ended = await door.closed;
  return ended === null ? { kind: "locked" } : { kind: "closed", code: ended };
}

function fresh(): Uint8Array<ArrayBuffer> {
  return crypto.getRandomValues(new Uint8Array(NONCE_BYTES));
}

export interface Door {
  readonly send: (bytes: Uint8Array<ArrayBuffer>) => void;

  readonly next: () => Promise<Uint8Array<ArrayBuffer> | null>;

  readonly closed: Promise<string | null>;
  readonly ended: () => Promise<Refusal>;
  readonly refuse: (why: Refused) => Refusal;
  readonly close: () => void;
}

function reach(url: string): Promise<Door | null> {
  const socket = new WebSocket(url);
  socket.binaryType = "arraybuffer";
  const arrived: Uint8Array<ArrayBuffer>[] = [];
  const waiting: ((bytes: Uint8Array<ArrayBuffer> | null) => void)[] = [];
  let shut = false;
  const closed = new Promise<string | null>((settle) => {
    socket.addEventListener("close", (event) => {
      shut = true;
      for (const wake of waiting.splice(0)) wake(null);
      settle(event.reason === "" ? null : event.reason);
    });
  });
  socket.addEventListener("message", (event: MessageEvent<unknown>) => {
    if (!(event.data instanceof ArrayBuffer)) return;
    const bytes = new Uint8Array(event.data);
    const wake = waiting.shift();
    if (wake === undefined) arrived.push(bytes);
    else wake(bytes);
  });
  const door: Door = {
    send: (bytes) => {
      socket.send(bytes);
    },
    next: () => {
      const ready = arrived.shift();
      if (ready !== undefined) return Promise.resolve(ready);
      if (shut) return Promise.resolve(null);
      return new Promise((wake) => {
        waiting.push(wake);
      });
    },
    closed,
    ended: async () => ({ kind: "closed", code: await closed }),
    refuse: (why) => {
      socket.close();
      return { kind: "refused", why };
    },
    close: () => {
      socket.close();
    },
  };
  return new Promise((settle) => {
    socket.addEventListener(
      "open",
      () => {
        settle(door);
      },
      { once: true },
    );
    socket.addEventListener(
      "error",
      () => {
        settle(null);
      },
      { once: true },
    );
  });
}

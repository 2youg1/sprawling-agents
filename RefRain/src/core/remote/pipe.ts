// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Door, Refusal } from "./connect";
import type { Refused } from "./handshake";

const HEADER_BYTES = 5;
const MAX_FRAME_BYTES = 64 * 1024 * 1024;
const MAX_READY_FRAMES = 8;
const PIPE_HEADER = "x-sprawling-pipe";
const BINARY = "application/octet-stream";

type FrameBytes = Uint8Array<ArrayBuffer>;

interface Packet {
  readonly type: number;
  readonly body: FrameBytes;
}

class Packets {
  private readonly header = new Uint8Array(HEADER_BYTES);
  private used = 0;
  private packet: Packet | null = null;

  *read(chunk: Uint8Array): Iterable<Packet | "invalid"> {
    let offset = 0;
    while (offset < chunk.length) {
      if (this.packet === null) {
        const length = Math.min(HEADER_BYTES - this.used, chunk.length - offset);
        this.header.set(chunk.subarray(offset, offset + length), this.used);
        this.used += length;
        offset += length;
        if (this.used < HEADER_BYTES) return;

        const head = new DataView(this.header.buffer);
        const type = head.getUint8(0);
        const bytes = head.getUint32(1);
        if (bytes > MAX_FRAME_BYTES || (type !== 0 && type !== 1)) {
          yield "invalid";
          return;
        }
        this.packet = { type, body: new Uint8Array(bytes) };
        this.used = 0;
      }

      const length = Math.min(this.packet.body.length - this.used, chunk.length - offset);
      this.packet.body.set(chunk.subarray(offset, offset + length), this.used);
      this.used += length;
      offset += length;
      if (this.used === this.packet.body.length) {
        const complete = this.packet;
        this.packet = null;
        this.used = 0;
        yield complete;
      }
    }
  }
}

export async function reach(url: string, first: FrameBytes): Promise<Door | null> {
  const request = new AbortController();
  const response = await fetch(url, {
    method: "POST",
    mode: "same-origin",
    credentials: "omit",
    redirect: "error",
    cache: "no-store",
    headers: { "content-type": BINARY },
    body: first,
    signal: request.signal,
  }).catch(() => null);
  if (response === null || !response.ok || response.body === null) {
    request.abort();
    return null;
  }
  const token = response.headers.get(PIPE_HEADER);
  if (token === null) {
    request.abort();
    return null;
  }
  return connected(new URL("/remote/frame", url), token, response.body, request);
}

function connected(
  url: URL,
  token: string,
  stream: ReadableStream<Uint8Array>,
  request: AbortController,
): Door {
  const ready: FrameBytes[] = [];
  const waiting: ((frame: FrameBytes | null) => void)[] = [];
  const reader = stream.getReader();
  const packets = new Packets();
  let shut = false;
  let outbound: Promise<void> = Promise.resolve();
  const { promise: closed, resolve: settled } = Promise.withResolvers<string | null>();

  function stop(code: string | null) {
    if (shut) return;
    shut = true;
    request.abort();
    for (const wake of waiting.splice(0)) wake(null);
    settled(code);
  }

  async function receive() {
    while (!shut) {
      const read = await reader.read().catch(() => null);
      if (read === null || read.done) break;
      for (const packet of packets.read(read.value)) {
        if (packet === "invalid") {
          stop("E_WIRE_MISMATCH");
          return;
        }
        if (packet.type === 1) {
          stop(new TextDecoder().decode(packet.body) || null);
          return;
        }
        const wake = waiting.shift();
        if (wake !== undefined) wake(packet.body);
        else if (ready.length < MAX_READY_FRAMES) ready.push(packet.body);
        else {
          stop("E_WIRE_MISMATCH");
          return;
        }
      }
    }
    stop("E_WIRE_MISMATCH");
  }

  void receive().finally(() => reader.releaseLock());
  return {
    send(bytes: FrameBytes) {
      outbound = outbound
        .then(async () => {
          if (shut) return;
          const answer = await fetch(url, {
            method: "POST",
            mode: "same-origin",
            credentials: "omit",
            redirect: "error",
            cache: "no-store",
            headers: { authorization: "Bearer " + token, "content-type": BINARY },
            body: bytes,
            signal: request.signal,
          });
          if (!answer.ok) stop("E_WIRE_MISMATCH");
        })
        .catch(() => stop("E_WIRE_MISMATCH"));
    },
    next() {
      const frame = ready.shift();
      if (frame !== undefined) return Promise.resolve(frame);
      if (shut) return Promise.resolve(null);
      return new Promise((wake) => waiting.push(wake));
    },
    closed,
    ended: async (): Promise<Refusal> => ({ kind: "closed", code: await closed }),
    refuse(why: Refused): Refusal {
      stop(null);
      return { kind: "refused", why };
    },
    close() {
      stop(null);
    },
  };
}

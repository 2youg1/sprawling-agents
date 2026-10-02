// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The line the link speaks through (client/Spec.lean §4-64): text frames out,
// text frames in, and the two moments it opens and closes. `socket.ts`
// drives the link over whichever line it is handed; this file holds the
// browser's own WebSocket to this origin's `/ws`, and
// `remote/session.ts` holds the sealed session through the remote door.

// The address of this city's socket, derived from the page's own origin:
// a client served by the city it talks to needs no configured endpoint.
export function socketUrl(location: Pick<Location, "protocol" | "host">): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}/ws`;
}

// The line the link speaks through: wire text out, and whether it went.
// `send` is false while the line is not open.
export interface Line {
  readonly send: (text: string) => boolean;
  readonly close: () => void;
}

// What a line tells the link. `closed` comes once per line, whether the
// line never opened or opened and ended.
export interface Hearing {
  readonly opened: () => void;
  readonly heard: (text: string) => void;
  readonly closed: () => void;
}

export type Dial = (hearing: Hearing) => Line;

// The browser's WebSocket to `url`, as a line.
export function plainDial(url: string): Dial {
  return (hearing) => {
    const socket = new WebSocket(url);
    let ended = false;
    const closed = () => {
      if (ended) return;
      ended = true;
      hearing.closed();
    };
    socket.onopen = hearing.opened;
    socket.onmessage = (message: MessageEvent) => {
      if (typeof message.data === "string") hearing.heard(message.data);
    };
    socket.onclose = closed;
    socket.onerror = closed;
    return {
      send: (text) => {
        if (socket.readyState !== WebSocket.OPEN) return false;
        socket.send(text);
        return true;
      },
      close: () => {
        socket.close();
      },
    };
  };
}

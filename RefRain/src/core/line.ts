// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


export function socketUrl(location: Pick<Location, "protocol" | "host">): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}/ws`;
}


export interface Line {
  readonly send: (text: string) => boolean;
  readonly close: () => void;
}


export interface Hearing {
  
  
  readonly opened: (credential: string | null) => void;
  readonly heard: (text: string) => void;
  readonly closed: () => void;
}

export type Dial = (hearing: Hearing) => Line;


export function plainDial(url: string): Dial {
  return (hearing) => {
    const socket = new WebSocket(url);
    let ended = false;
    const closed = () => {
      if (ended) return;
      ended = true;
      hearing.closed();
    };
    socket.onopen = () => {
      hearing.opened(null);
    };
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

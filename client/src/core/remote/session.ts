// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The whole client speaking through the remote door (client/Spec.lean §4-64,
// `crates/remote_access/Spec.lean` §8-10): a page opened from the remote
// address, in an origin that kept a paired device, speaks the wire as
// sealed payloads on `/remote/session`; any other page speaks to this
// origin's `/ws`. Which line a connection takes is judged each time the
// link connects, so a device paired on this very page speaks through
// the door from the link's next attempt on.
//
// WebCrypto seals and opens asynchronously, and two calls may finish
// out of turn, while the city opens frames by count. So the frames this
// line sends are sealed one after another, each sent before the next is
// sealed, and the frames it receives are opened one after another.

import { plainDial, socketUrl } from "../line";
import type { Dial, Hearing, Line } from "../line";
import { remoteUrl, sessionOver } from "./connect";
import type { Door } from "./connect";
import { kept } from "./device";
import type { Session } from "./handshake";
import { payloadBytes, payloadOf } from "./seal";

// The line for each connection the link opens from `location`.
export function dialFor(location: Pick<Location, "protocol" | "host">): Dial {
  const plain = plainDial(socketUrl(location));
  const url = remoteUrl(location, "/remote/session");
  if (url === null) return plain;
  return (hearing) => {
    let line: Line | null = null;
    let dropped = false;
    // Read through a call: the link may drop the line while the device
    // record or the handshake is awaited.
    const gone = (): boolean => dropped;
    void kept().then(async (device) => {
      if (gone()) return;
      if (device === null) {
        line = plain(hearing);
        return;
      }
      const opened = await sessionOver(url, device);
      if (opened.kind !== "opened") {
        if (!gone()) hearing.closed();
        return;
      }
      if (gone()) {
        opened.door.close();
        return;
      }
      line = talk(opened.door, opened.session, hearing);
    });
    return {
      send: (text) => line?.send(text) ?? false,
      close: () => {
        if (line !== null) {
          line.close();
          return;
        }
        if (dropped) return;
        dropped = true;
        hearing.closed();
      },
    };
  };
}

// A session the door admitted, as a line: open from the start, closed
// once - when the city ends the connection, when a frame does not open
// or is not a wire frame, when a frame will not seal, or when the link
// closes it.
export function talk(door: Door, session: Session, hearing: Hearing): Line {
  let open = true;
  // Read through a call: `end` may run while a frame is awaited.
  const isOpen = (): boolean => open;
  let sending = Promise.resolve();
  const end = (): void => {
    if (!open) return;
    open = false;
    door.close();
    hearing.closed();
  };
  hearing.opened();
  void (async () => {
    for (let sealed = await door.next(); sealed !== null && isOpen(); sealed = await door.next()) {
      const bytes = await session.opener.open(sealed);
      const payload = bytes === null ? null : payloadOf(bytes);
      if (payload?.kind !== "frame") break;
      if (isOpen()) hearing.heard(payload.text);
    }
    end();
  })();
  return {
    send: (text) => {
      if (!open) return false;
      sending = sending.then(async () => {
        const sealed = await session.sealer.seal(payloadBytes({ kind: "frame", text }));
        if (sealed === null) end();
        else if (isOpen()) door.send(sealed);
      });
      return true;
    },
    close: end,
  };
}

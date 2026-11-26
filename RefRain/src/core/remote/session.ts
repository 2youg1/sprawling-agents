// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { plainDial, socketUrl } from "../line";
import { renewing } from "../local/dial";
import type { Credential } from "../local/entering";
import type { Dial, Hearing, Line } from "../line";
import { remoteUrl, sessionOver } from "./connect";
import type { Door } from "./connect";
import { kept } from "./device";
import type { Session } from "./handshake";
import { payloadBytes, payloadOf } from "./seal";

export function dialFor(
  location: Pick<Location, "protocol" | "host">,
  credential: Credential,
  forgotten: () => void,
): Dial {
  const plain = renewing(plainDial(socketUrl(location)), credential, forgotten);
  const url = remoteUrl(location, "/remote/session");
  if (url === null) return plain;
  return (hearing) => {
    let line: Line | null = null;
    let dropped = false;

    const gone = (): boolean => dropped;
    const closeUnopened = (): void => {
      if (dropped) return;
      dropped = true;
      hearing.closed();
    };
    void kept().then(async (device) => {
      if (gone()) return;
      if (device === null) {
        closeUnopened();
        return;
      }
      const opened = await sessionOver(url, device);
      if (opened.kind !== "opened") {
        closeUnopened();
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
        if (line === null) closeUnopened();
        else line.close();
      },
    };
  };
}

export function talk(door: Door, session: Session, hearing: Hearing): Line {
  let open = true;

  const isOpen = (): boolean => open;
  let sending = Promise.resolve();
  const end = (): void => {
    if (!open) return;
    open = false;
    door.close();
    hearing.closed();
  };
  queueMicrotask(() => {
    if (isOpen()) hearing.opened(null);
  });
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

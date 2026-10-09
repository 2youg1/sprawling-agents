// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A line that renews the page's credential before it dials, so every
// socket attempt shows the city a token minted for it: a token outlives
// the socket it opened only by the city's idle allowance, and a city
// that restarted knows no token at all, while it still knows the
// device key that signs for a new one.

import type { Dial, Line } from "../line";
import type { Credential } from "./entering";

export function renewing(dial: Dial, credential: Credential, forgotten: () => void): Dial {
  return (hearing) => {
    let line: Line | null = null;
    let dropped = false;
    // Read through a call: the link may drop the line while the renewal
    // is awaited.
    const gone = (): boolean => dropped;
    void credential.renew().then((renewed) => {
      if (gone()) return;
      if (renewed === "forgotten") {
        dropped = true;
        hearing.closed();
        forgotten();
        return;
      }
      line = dial({
        ...hearing,
        opened: () => {
          hearing.opened(credential.current());
        },
      });
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

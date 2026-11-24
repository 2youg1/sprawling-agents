// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Dial, Line } from "../line";
import type { Credential } from "./entering";

export function renewing(dial: Dial, credential: Credential, forgotten: () => void): Dial {
  let initial = credential.current();
  return (hearing) => {
    let line: Line | null = null;
    let dropped = false;
    const gone = (): boolean => dropped;
    const token = initial;
    initial = null;
    const ready = token === null ? credential.renew() : Promise.resolve("kept" as const);
    void ready.then((renewed) => {
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
          hearing.opened(token ?? credential.current());
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

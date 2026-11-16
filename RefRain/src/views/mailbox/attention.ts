// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Readable } from "svelte/store";

import type { Attention, Box } from "../../core/deferral";
import type { Conversing } from "../talk/handing";

export interface Attending {
  readonly read: () => Attention;
  readonly stop: () => void;
}

function composerBox(): HTMLTextAreaElement | null {
  const box = document.querySelector("main textarea");
  return box instanceof HTMLTextAreaElement ? box : null;
}

export function attending(
  conversing: Readable<Conversing>,
  now: () => number,
  changed: () => void,
): Attending {
  let heard: Box = { kind: "absent" };
  let from: HTMLTextAreaElement | null = null;
  let returnedAt: number | null = null;

  function fresh(box: HTMLTextAreaElement): Box {
    return box.value === "" ? { kind: "emptied", at: now(), by: "hand" } : { kind: "holding" };
  }

  function read(): Attention {
    const box = composerBox();
    if (box === null) {
      from = null;
      heard = { kind: "absent" };
    } else if (box !== from) {
      from = box;
      heard = fresh(box);
    }
    return { box: heard, visible: document.visibilityState === "visible", returnedAt };
  }

  function typed(event: Event): void {
    const box = composerBox();
    if (box === null || event.target !== box) return;
    from = box;
    const next = fresh(box);
    if (next.kind === heard.kind) return;
    heard = next;
    changed();
  }

  function shown(): void {
    if (document.visibilityState !== "visible") return;
    returnedAt = now();
    changed();
  }

  let last: Conversing | undefined;
  const unsubscribe = conversing.subscribe((next) => {
    const sent = last !== undefined && next.kind === "waiting" && next !== last;
    last = next;
    if (!sent) return;
    from = composerBox();
    heard = { kind: "emptied", at: now(), by: "send" };
    changed();
  });

  document.addEventListener("input", typed, true);
  document.addEventListener("visibilitychange", shown);

  return {
    read,
    stop() {
      unsubscribe();
      document.removeEventListener("input", typed, true);
      document.removeEventListener("visibilitychange", shown);
    },
  };
}

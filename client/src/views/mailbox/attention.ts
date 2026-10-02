// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The page read into the `Attention` that `core/deferral.ts` judges: the
// composer's box, whether the tab is shown, and when it was last shown
// again. The judgement is that file's; this one only listens.
//
// **The box is found the way the shell finds it** - the one textarea
// under `main` (`app.svelte`'s `focusComposer`) - so no view has to
// report its own typing. Typing is heard as the `input` events that
// bubble from it; a send is heard as the conversation saying it handed
// words to the city (`ui.conversing`, written by the talk page alone),
// because a send empties the box without an `input` event.

import type { Readable } from "svelte/store";

import type { Attention, Box } from "../../core/deferral";
import type { Conversing } from "../talk/handing";

export interface Attending {
  // The page as it stands now.
  readonly read: () => Attention;
  readonly stop: () => void;
}

function composerBox(): HTMLTextAreaElement | null {
  const box = document.querySelector("main textarea");
  return box instanceof HTMLTextAreaElement ? box : null;
}

// `changed` is called whenever a moment may have opened: a send, a box
// emptied or filled, the tab shown again.
export function attending(conversing: Readable<Conversing>, now: () => number, changed: () => void): Attending {
  // The box as last heard, and which textarea it was heard from: a
  // composer that mounts after a navigation is a new box, read afresh.
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
    // Typing into a box that was already empty keeps the moment it
    // began; only a change of kind is news.
    if (next.kind === heard.kind) return;
    heard = next;
    changed();
  }

  function shown(): void {
    if (document.visibilityState !== "visible") return;
    returnedAt = now();
    changed();
  }

  // The store answers once as it is subscribed, with whatever it held
  // before this page listened; that is not a send heard here.
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

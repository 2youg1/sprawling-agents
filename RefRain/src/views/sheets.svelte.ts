// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { closeRight, rightItem } from "./inspect/open.svelte";

export type Sheet = "world" | "right";


const KEY = "sheets";

let open = $state.raw<readonly Sheet[]>([]);


let opener: HTMLElement | null = null;

function sheetsIn(state: unknown): readonly Sheet[] {
  if (typeof state !== "object" || state === null || !(KEY in state)) return [];
  const listed: unknown = state[KEY];
  if (!Array.isArray(listed)) return [];
  return listed.flatMap((each: unknown): Sheet[] =>
    each === "world" || each === "right" ? [each] : [],
  );
}


export function sheetsOpen(): readonly Sheet[] {
  return open;
}


export function openSheet(sheet: Sheet): void {
  if (open.at(-1) === sheet) return;
  if (sheet === "world")
    opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const next = [...open.filter((each) => each !== sheet), sheet];
  history.pushState({ [KEY]: next }, "");
  open = next;
}


export function leaveSheet(sheet: Sheet): void {
  if (open.at(-1) === sheet) history.back();
}


export function keepSheets(): () => void {
  const read = (): void => {
    const was = open;
    const held = sheetsIn(history.state);
    if (was.includes("right") && !held.includes("right")) closeRight();
    open = rightItem() === null ? held.filter((sheet) => sheet !== "right") : held;
    if (open.length !== held.length) history.replaceState({ [KEY]: open }, "");
    if (was.includes("world") && !open.includes("world")) {
      queueMicrotask(() => {
        if (document.activeElement === document.body && opener?.isConnected === true)
          opener.focus();
        opener = null;
      });
    }
  };
  read();
  window.addEventListener("popstate", read);
  window.addEventListener("hashchange", read);
  return () => {
    window.removeEventListener("popstate", read);
    window.removeEventListener("hashchange", read);
  };
}

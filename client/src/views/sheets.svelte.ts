// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The full-screen sheets of a one-column shell (client-SPEC 4-52, refrain
// U9): the world layer, entering from the left, and the right side,
// entering from the right. Each open sheet is an entry of the browser's
// own history, so the sheet's back key, the browser's back button and an
// edge swipe back all walk one stack, and leaving a sheet returns to what
// was under it (refrain §3-14, focus and return). Which sheets are open
// is read off the entry the browser stands on, here, and nowhere else.

import { closeRight, rightItem } from "./inspect/open.svelte";

export type Sheet = "world" | "right";

// The key of the history entry's state that lists its open sheets,
// bottom first.
const KEY = "sheets";

let open = $state.raw<readonly Sheet[]>([]);
// What had the focus when the world sheet opened, which takes it back
// when the sheet closes with the focus lost inside it (client-SPEC 7-7).
// The right side keeps its own opener (`inspect/open.svelte.ts`).
let opener: HTMLElement | null = null;

function sheetsIn(state: unknown): readonly Sheet[] {
  if (typeof state !== "object" || state === null || !(KEY in state)) return [];
  const listed: unknown = state[KEY];
  if (!Array.isArray(listed)) return [];
  return listed.flatMap((each: unknown): Sheet[] => (each === "world" || each === "right" ? [each] : []));
}

// The sheets the current history entry holds open, bottom first.
export function sheetsOpen(): readonly Sheet[] {
  return open;
}

// Opens `sheet` over whatever is open, as a new history entry; a sheet
// already on top is left as it is.
export function openSheet(sheet: Sheet): void {
  if (open.at(-1) === sheet) return;
  if (sheet === "world") opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const next = [...open.filter((each) => each !== sheet), sheet];
  history.pushState({ [KEY]: next }, "");
  open = next;
}

// Steps back off `sheet` when it is the one on top; the browser's
// `popstate` then says what is open under it.
export function leaveSheet(sheet: Sheet): void {
  if (open.at(-1) === sheet) history.back();
}

// Reads the open sheets from the entry the browser stands on now and
// after every move through its history, until the returned function is
// called. The shell calls it once. A sheet the move left is closed: the
// right side lets its items go, and focus lost inside the world sheet
// goes back to what opened it. An entry that names the right side when
// nothing is open on it any more - after a reload, or back from a page a
// link in it led to - is rewritten without it, rather than stepped back
// off, which would carry the person one page further than they asked.
export function keepSheets(): () => void {
  const read = (): void => {
    const was = open;
    const held = sheetsIn(history.state);
    if (was.includes("right") && !held.includes("right")) closeRight();
    open = rightItem() === null ? held.filter((sheet) => sheet !== "right") : held;
    if (open.length !== held.length) history.replaceState({ [KEY]: open }, "");
    if (was.includes("world") && !open.includes("world")) {
      queueMicrotask(() => {
        if (document.activeElement === document.body && opener?.isConnected === true) opener.focus();
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

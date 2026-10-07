<script lang="ts">
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How a key is drawn, in the two places a key is ever drawn: beside the
// thing it reaches, and all together on the sheet `?` opens. Both read
// `core/keys`, so a rebind moves the mark beside a control and the row on
// the sheet at once, and neither file spells a key of its own. The same
// mark draws a list's own keys at a row's end: a line move, or the first
// letter a row is reached by.
//
// This file is the seat of both: it reads the key table and holds the
// sheet's element, and `kbd.look.svelte` and `kbd_sheet.look.svelte`
// draw.
//
// **The sheet is a modal dialog the platform owns.** `showModal()` puts
// it in the top layer, traps the focus, marks the rest of the page
// `inert`, and answers Escape - four behaviours this file used to carry
// as an overlay, a focus call and a stacking order. There is therefore
// no `z-index` here (design 4-21) and no focus code: `close()` at
// teardown returns the focus to whatever held it before the sheet
// opened (SPEC 7-7).
//
// **The sheet has two seats.** `modal` is the one above; `specimen` is
// the same box drawn open in the page's flow, with no `showModal()`, for
// `#/gallery`, where a top-layer sheet would cover every other fold of
// the route and dim it behind a backdrop. The specimen takes no focus,
// marks nothing `inert` and answers no Escape, because nothing on that
// route asked a question of it.
//
// A caller writes `import Cheatsheet, { Kbd } from ".../kbd.svelte"`:
// `Kbd` is the exported snippet, one glyph per `<kbd>`, no focus and no
// keys of its own (SPEC 7-1).

import type { Attachment } from "svelte/attachments";

import { ACTIONS, LABELS } from "../../core/keys";
import { say } from "../../core/lang";
import { ui } from "../../ui";
import Look from "./kbd_sheet.look.svelte";
import { sheetOf, type Seat } from "./kbd";

interface CheatsheetProps {
  readonly seat: Seat;
  readonly onClose: () => void;
}

const uid = $props.id();
const { lang } = ui();
const { seat, onClose }: CheatsheetProps = $props();

let sheet = $state<HTMLDialogElement | undefined>(undefined);
const hold: Attachment<HTMLDialogElement> = (node) => {
  sheet = node;
  return () => {
    sheet = undefined;
  };
};

// The caller owns whether the sheet stands; this only carries that
// answer to the element. Asking an already-open dialog to open throws,
// so the call is made once, on the edge this component's life gives it.
$effect(() => {
  const node = sheet;
  if (node === undefined || seat === "specimen") {
    return;
  }
  node.showModal();
  return () => {
    node.close();
  };
});

const look = $derived(
  sheetOf(
    { uid, seat, onClose, hold },
    {
      title: say($lang, "keys_title"),
      dismiss: say($lang, "dismiss"),
      where: say($lang, "keys_where"),
      rows: ACTIONS.map((action) => ({
        key: action,
        label: say($lang, LABELS[action]),
        marks: chordMarks({ action }),
      })),
    },
  ),
);
</script>

<script module>
import type { Action } from "../../core/keys";
import { keymap, marks } from "../../core/keys";
import type { LineMove } from "../../core/lines";
import { lineFaces } from "../../core/lines";
import KbdLook from "./kbd.look.svelte";

// What is drawn: the chord of a shell action, the key a row draws for a
// line move (the first of that move's keys), or a row's first letter.
export type KbdProps = (
  | { readonly action: Action }
  | { readonly move: LineMove }
  | { readonly initial: string }
) & { readonly class?: string };

export { Kbd };

// The chord that reaches one action, as this machine writes it: `⌘` on
// a Mac and `Ctrl` everywhere else, and a prefixed chord as the two
// keystrokes it is.
function chordMarks(props: KbdProps): readonly string[] {
  const keys = keymap();
  if ("move" in props) return lineFaces(props.move).slice(0, 1);
  if ("initial" in props) return marks({ accel: false, shift: false, key: props.initial }, keys.platform);
  return marks(keys.chord(props.action), keys.platform);
}
</script>

{#snippet Kbd(props: KbdProps)}
  <KbdLook marks={chordMarks(props)} class={props.class} />
{/snippet}

<Look {...look} />

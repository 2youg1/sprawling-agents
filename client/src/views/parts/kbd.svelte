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
// **The sheet is a modal dialog the platform owns.** `showModal()` puts
// it in the top layer, traps the focus, marks the rest of the page
// `inert`, and answers Escape - four behaviours this file used to carry
// as an overlay, a focus call and a stacking order. There is therefore
// no `z-index` here (design 4-21) and no focus code: `close()` at
// teardown returns the focus to whatever held it before the sheet
// opened (SPEC 7-7). Escape arrives as a `cancel` request, which is
// answered by the caller rather than by the element, and a click on the
// backdrop closes nothing - a sheet of key chords is not a question one
// answers by missing it (SPEC 7-3).
//
// A caller writes `import Cheatsheet, { Kbd } from ".../kbd.svelte"`:
// `Kbd` is the exported snippet, one glyph per `<kbd>`, no focus and no
// keys of its own (SPEC 7-1).

import { ACTIONS, LABELS } from "../../core/keys";
import { say } from "../../core/lang";
import { ui } from "../../ui";

interface CheatsheetProps {
  readonly onClose: () => void;
}

// The sheet's own box, in the shape `parts/dialog.svelte` draws: a
// shadowed face draws no border, it fades in through `@starting-style`
// and out through `transition-behavior: allow-discrete`, moves only
// opacity, and goes still under `prefers-reduced-motion`. The seat is
// the one the sheet has always had - a section below the top edge,
// centred across the page - because reading twelve chords is not
// navigating and the page underneath should not have to move to be read
// over.
const SHEET =
  "m-auto mt-section hidden w-full max-w-measure flex-col gap-base rounded-panel " +
  "bg-raised p-pane opacity-0 shadow-sheet transition-[opacity,display,overlay] " +
  "transition-discrete duration-panel ease-leave open:flex open:opacity-100 open:ease-arrive " +
  "starting:open:opacity-0 motion-reduce:transition-none " +
  "backdrop:bg-transparent backdrop:backdrop-brightness-50";

const uid = $props.id();
const { lang } = ui();
const { onClose }: CheatsheetProps = $props();

let sheet = $state<HTMLDialogElement | undefined>(undefined);

// The caller owns whether the sheet stands; this only carries that
// answer to the element. Asking an already-open dialog to open throws,
// so the call is made once, on the edge this component's life gives it.
$effect(() => {
  const node = sheet;
  if (node === undefined) {
    return;
  }
  node.showModal();
  return () => {
    node.close();
  };
});
</script>

<script module>
import type { Action } from "../../core/keys";
import { keymap, marks } from "../../core/keys";
import type { LineMove } from "../../core/lines";
import { lineFaces } from "../../core/lines";

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
  <span class={["inline-flex items-center gap-tight", props.class]}>
    {#each chordMarks(props) as mark (mark)}
      <kbd
        class="rounded-control px-tight font-mono text-note leading-none text-text-quiet no-underline"
        >{mark}</kbd
      >
    {/each}
  </span>
{/snippet}

<dialog
  bind:this={sheet}
  class={SHEET}
  aria-labelledby="{uid}-title"
  aria-describedby="{uid}-where"
  oncancel={(event) => {
    // Escape reaches here as a cancel request. The default would close
    // the element behind the caller's back, so the request is handed to
    // the caller instead (SPEC 7-3).
    event.preventDefault();
    onClose();
  }}
>
  <div class="mb-base flex items-baseline justify-between">
    <h2 id="{uid}-title" class="text-heading font-heading text-text">{say($lang, "keys_title")}</h2>
    <button
      type="button"
      class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-raised hover:text-text"
      onclick={() => {
        onClose();
      }}
    >
      {say($lang, "dismiss")}
    </button>
  </div>
  <ul class="flex flex-col">
    {#each ACTIONS as action (action)}
      <li class="flex items-center justify-between gap-base py-tight text-label">
        <span class="truncate text-text-quiet">{say($lang, LABELS[action])}</span>
        <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
        {@render Kbd({ action })}
      </li>
    {/each}
  </ul>
  <p id="{uid}-where" class="mt-base text-note text-text-faint">{say($lang, "keys_where")}</p>
</dialog>

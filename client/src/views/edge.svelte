<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of the three edge keys at the foot of the first column
  // (docs/frontend-method.md §7E): layers, which changes the tier; the
  // mailbox; settings. They are the only standing buttons on the page
  // besides the composer's, and nothing stands above them.
  //
  // This file owns the layers key's press, which may become a hold, and
  // the column the keys stand in; what each key says and how it is
  // wired is `./edge.ts`, and how it is drawn is whatever
  // `./edge_key.look.svelte` is. Every word and every chord is read from
  // `core/keys`, so a rebind shows here without this file knowing what
  // was pressed.
  import { keymap, marks } from "../core/keys";
  import type { Action } from "../core/keys";
  import { HOLD_MS } from "../core/press";
  import { say } from "../core/lang";
  import type { Tier } from "../core/prefs";
  import { toFragment } from "../core/route";
  import { ui } from "../ui";
  import { layersKey, settingsKey } from "./edge";
  import Look from "./edge_key.look.svelte";
  import Mailbox from "./mailbox/mailbox.svelte";

  interface Props {
    // The tier the page is drawn in now, which is the person's own
    // unless the layers key is being held.
    readonly tier: Tier;
    readonly onTier: () => void;
    // Holding the layers key shows the blend tier until it is let go.
    readonly onPeek: (peeking: boolean) => void;
    // How many times the mailbox key asked for the mailbox.
    readonly mailboxAsked: number;
  }

  const { tier, onTier, onPeek, mailboxAsked }: Props = $props();

  const { lang } = ui();
  const keys = keymap();
  const bound = keys.bound;

  // The words a key's hint carries: its name, then the chord that
  // reaches it as this platform writes it.
  function named(action: Action): (words: string) => string {
    return (words) => {
      const drawn = marks($bound[action], keys.platform);
      return drawn.length === 0 ? words : `${words}  ${drawn.join(" ")}`;
    };
  }

  // A press on the layers key is a change of tier unless it was held
  // long enough to be a look: the same hand that taps to switch holds to
  // peek, and letting go of a peek must not switch as well.
  let holding: ReturnType<typeof setTimeout> | undefined;
  let peeked = false;

  // The press keeps the focus where it was: a soft keyboard open over the
  // composer stays open while the tier changes (refrain §3-14).
  function press(event: PointerEvent): void {
    event.preventDefault();
    peeked = false;
    holding = setTimeout(() => {
      peeked = true;
      onPeek(true);
    }, HOLD_MS);
  }

  function release(): void {
    clearTimeout(holding);
    holding = undefined;
    if (peeked) onPeek(false);
  }

  function click(): void {
    if (peeked) {
      peeked = false;
      return;
    }
    onTier();
  }
</script>

<nav
  class="edge-keys relative z-10 col-start-1 row-start-2 flex flex-col items-start gap-snug self-end narrow:row-start-3 narrow:flex-row narrow:pt-snug"
  aria-label={say($lang, "region_edge")}
>
  <Look {...layersKey(tier, $lang, named("tier.cycle"), { press, release, click })} />
  <Mailbox asked={mailboxAsked} hint={named("mailbox")} />
  <Look {...settingsKey($lang, named("go.setup"), toFragment({ kind: "setup" }))} />
</nav>

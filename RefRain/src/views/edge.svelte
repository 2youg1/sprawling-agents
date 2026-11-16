<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
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
    readonly tier: Tier;
    readonly onTier: () => void;
    readonly onPeek: (peeking: boolean) => void;
    readonly mailboxAsked: number;
  }

  const { tier, onTier, onPeek, mailboxAsked }: Props = $props();

  const { lang } = ui();
  const keys = keymap();
  const bound = keys.bound;

  function named(action: Action): (words: string) => string {
    return (words) => {
      const drawn = marks($bound[action], keys.platform);
      return drawn.length === 0 ? words : `${words}  ${drawn.join(" ")}`;
    };
  }

  let holding: ReturnType<typeof setTimeout> | undefined;
  let peeked = false;

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
  class="edge-keys relative z-(--layer-edge) col-start-1 row-start-2 flex flex-col items-start gap-snug self-end narrow:row-start-3 narrow:flex-row narrow:pt-snug"
  aria-label={say($lang, "region_edge")}
>
  <Look {...layersKey(tier, $lang, named("tier.cycle"), { press, release, click })} />
  <Mailbox asked={mailboxAsked} hint={named("mailbox")} />
  <Look {...settingsKey($lang, named("go.setup"), toFragment({ kind: "setup" }))} />
</nav>

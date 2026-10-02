<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // The three edge keys at the foot of the first column (client-SPEC 7E):
  // layers, which changes the tier; the mailbox; settings. They are the
  // only standing buttons on the page besides the composer's, and
  // nothing stands above them.
  //
  // A key's name and its chord are not drawn until somebody looks: they
  // stand to the key's right on hover and on focus, and all three come
  // at once while the accelerator is held alone (`app.svelte` sets the
  // root's `data-expose`, `parts/tip.svelte` answers it). Every word and
  // every chord is read from `core/keys`, so a rebind shows here without
  // this file knowing what was pressed.

  // The key every edge control is drawn as: a glass square with a
  // rounder corner than a control, and a lift under the pointer.
  export const EDGE_KEY =
    "glass relative grid size-key place-items-center rounded-key text-text transition-transform hover:-translate-y-px";
</script>

<script lang="ts">
  import { keymap, marks } from "../core/keys";
  import type { Action } from "../core/keys";
  import { say } from "../core/lang";
  import type { Key } from "../core/lang";
  import { TIERS } from "../core/prefs";
  import type { Tier } from "../core/prefs";
  import { toFragment } from "../core/route";
  import { ui } from "../ui";
  import Mailbox from "./mailbox/mailbox.svelte";
  import Glyph from "./parts/glyph.svelte";
  import Tip from "./parts/tip.svelte";

  interface Props {
    // The tier the page is drawn in now, which is the person's own
    // unless the layers key is being held.
    readonly tier: Tier;
    readonly onTier: () => void;
    // Holding the layers key shows the blend tier until it is let go.
    readonly onPeek: (peeking: boolean) => void;
    // How many times Accel-B asked for the mailbox.
    readonly mailboxAsked: number;
  }

  const { tier, onTier, onPeek, mailboxAsked }: Props = $props();

  const { lang } = ui();
  const keys = keymap();
  const bound = keys.bound;

  const TIER_NAME: Record<Tier, Key> = {
    zen: "tier_zen",
    blend: "tier_blend",
    panorama: "tier_panorama",
  };

  // The words a key's hint carries: its name, then the chord that
  // reaches it as this platform writes it.
  function named(words: string, action: Action): string {
    return `${words}  ${marks($bound[action], keys.platform).join(" ")}`;
  }

  // A press on the layers key is a change of tier unless it was held
  // long enough to be a look: the same hand that taps to switch holds to
  // peek, and letting go of a peek must not switch as well.
  const HOLD_MS = 300;
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
  <Tip text={named(`${say($lang, "edge_layers")} · ${say($lang, TIER_NAME[tier])}`, "tier.cycle")} side="right" exposable>
    {#snippet children(hint)}
      <button
        type="button"
        class={EDGE_KEY}
        aria-label={`${say($lang, "edge_layers")} · ${say($lang, TIER_NAME[tier])}`}
        aria-describedby={hint}
        onpointerdown={press}
        onpointerup={release}
        onpointerleave={release}
        onclick={click}
      >
        <Glyph name="layers" size="key" class="-mt-[3px]" />
        <span class="absolute bottom-[5px] left-1/2 flex -translate-x-1/2 gap-[3px]" aria-hidden="true">
          {#each TIERS as each (each)}
            <i class={["h-[2px] w-[4px] rounded-pill", each === tier ? "bg-accent" : "bg-mark"]}></i>
          {/each}
        </span>
      </button>
    {/snippet}
  </Tip>
  <Mailbox asked={mailboxAsked} hint={(words: string) => named(words, "mailbox")} />
  <Tip text={named(say($lang, "nav_settings"), "go.setup")} side="right" exposable>
    {#snippet children(hint)}
      <a
        href={toFragment({ kind: "setup" })}
        class={EDGE_KEY}
        aria-label={say($lang, "nav_settings")}
        aria-describedby={hint}
      >
        <Glyph name="settings" size="key" />
      </a>
    {/snippet}
  </Tip>
</nav>

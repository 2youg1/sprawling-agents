<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Key } from "../../core/lang";
  import table from "../../lang.json";

  // Something that happened, against something that was refused.
  export type Weight = "info" | "alert";

  // Hugging the field it belongs to, floating over the page for a few
  // seconds, or standing in the drawer that keeps it (client-SPEC
  // 4-35). The seat changes the drawing and nothing else: the role is
  // the weight's to decide (7-1).
  export type Seat = "inline" | "toast" | "drawer";

  // Each seat carries the one entrance the motion vocabulary grants it:
  // a toast is a popover, and a popover rises into place; a strip in a
  // form and an entry in a list are state changes that keep their
  // position, so they fade. `theme.css` stills every one of these under
  // the two reduction lists. There is deliberately no exit class: a
  // leaving notice is removed by whoever keeps the list, and that owner
  // animates the departure through `allow-discrete` plus a display
  // toggle if it wants one at all.
  const PAINT: Record<Seat, string> = {
    toast:
      "rise w-[min(480px,calc(100vw_-_var(--spacing-rail-open)_-_2_*_var(--spacing-pane)))] " +
      "rounded-panel border border-edge-panel bg-raised px-pane py-base shadow-float",
    inline: "fade mt-tight rounded-card border border-edge-input px-snug py-tight",
    drawer: "fade w-full border-b border-edge px-base py-snug",
  };

  // The title is the sentence this client can say in the reader's own
  // language, found under `err_` plus the code in lower case. A code
  // whose key `lang.json` does not hold has no translation yet, and the
  // title then shows that key name itself: a visible `err_e_foo` is a
  // defect somebody reports, and a silently chosen default sentence is
  // one nobody does (`fill` leaves its unfilled slots visible for the
  // same reason).
  function isKey(raw: string): raw is Key {
    return Object.hasOwn(table, raw);
  }
</script>

<script lang="ts">
  // Something that happened, said once where a person is looking and
  // kept where they can look again. One component draws every notice
  // the client has - inline beside a field, a toast in the corner, an
  // entry in the notification drawer - so a notice cannot be shown in
  // one place and shaped differently in another (client-SPEC 4-35).
  //
  // **The reader's own title is the heading; the city's way out is the
  // body; the rest of its words are the fold.** The heading comes from
  // `err_<code>` in `lang.json`, and one code covers several causes, so
  // the heading names the kind of refusal and never the cause. The
  // recovery the city wrote is the one sentence that knows the cause
  // and what to do about it, so it stands under the heading unfolded; a
  // person who had to open a disclosure to learn that the next step is
  // the settings page would first press the buttons that do not help.
  // The action and the subject fold into one mono-font disclosure and
  // never stand as a heading (UX B8). The stable code itself rides the
  // heading's right edge, where a person can cite it.
  //
  // **The same refusal is one notice with a count.** However many times
  // a refusal arrives, a person who reads the same four fields twice
  // cannot tell a city that failed once from one that failed twenty
  // times. The merging happens where the notices are kept, keyed by the
  // code and the subject together - one code covers several subjects,
  // and the subject is what a person acts on; this component receives
  // the strings and the count and draws them.
  import type { Snippet } from "svelte";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "./badge.svelte";

  interface Props {
    readonly seat: Seat;
    // Absent is `info`: `alert` is for something that was refused.
    readonly weight?: Weight | undefined;
    // The error, as the city wrote it: the action that failed, its
    // stable code, what it was against, and what the caller can do next.
    // Strings rather than the belief record, so this draws refusals from
    // anywhere the wire carries one.
    readonly action: string;
    readonly code: string;
    readonly subject: string;
    readonly recovery: string;
    // Already formatted by the caller's clock.
    readonly at?: string | undefined;
    // How many times this refusal arrived; the badge shows past one.
    readonly count?: number | undefined;
    // Usually quiet Buttons carrying the recovery verbs.
    readonly actions?: Snippet | undefined;
  }

  const { seat, weight, action, code, subject, recovery, at, count, actions }: Props = $props();

  const { lang } = ui();

  const titleKey = $derived(`err_${code.toLowerCase()}`);
  const title = $derived(isKey(titleKey) ? say($lang, titleKey) : titleKey);
</script>

<div
  role={weight === "alert" ? "alert" : "status"}
  class={[PAINT[seat], "flex min-w-0 items-start justify-between gap-base text-note"]}
>
  <div class="flex min-w-0 flex-col gap-tight">
    <div class="flex min-w-0 items-baseline gap-snug">
      <span class={["truncate font-label", weight === "alert" ? "text-alert" : "text-text"]}>
        {title}
      </span>
      {#if at !== undefined}
        <span class="shrink-0 text-text-faint">{at}</span>
      {/if}
      {#if count !== undefined && count > 1}
        <Badge text={`×${String(count)}`} weight="quiet" />
      {/if}
      <span class="ml-auto shrink-0 font-mono text-note text-text-faint">{code}</span>
    </div>
    {#if recovery !== ""}
      <p class="min-w-0 wrap-anywhere text-note text-text">{recovery}</p>
    {/if}
    <details class="min-w-0">
      <summary class="cursor-pointer text-note text-text-faint">{say($lang, "notices_detail")}</summary>
      <div
        class="mt-tight flex min-w-0 flex-col gap-tight wrap-anywhere font-mono text-note text-text-quiet"
      >
        <span>{action}</span>
        <span>{subject}</span>
      </div>
    </details>
  </div>
  {#if actions}
    <div class="flex shrink-0 items-center gap-tight">{@render actions()}</div>
  {/if}
</div>

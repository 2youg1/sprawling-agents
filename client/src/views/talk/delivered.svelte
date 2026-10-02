<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The words a person just sent, drawn at the foot of the thread the
moment they leave the box, with where they stand (`delivery.ts`). It is
the page's own echo, so it is marked `data-local-feedback` and drawn
quieter than a message the city holds; the run's thread replaces it with
the same words as its task once the city starts the run. -->
<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Delivery } from "./delivery";

  interface Props {
    readonly delivery: Exclude<Delivery, { readonly kind: "none" }>;
  }

  const { delivery }: Props = $props();

  const { lang } = ui();

  const state = $derived.by(() => {
    switch (delivery.kind) {
      case "held":
        return say($lang, "talk_delivery_held");
      case "pending":
        return say($lang, "talk_delivery_pending");
      case "accepted":
        return say($lang, "talk_delivery_accepted");
      case "unknown":
        return say($lang, "talk_delivery_unknown");
    }
  });
</script>

<div class="my-base flex flex-col items-end" data-local-feedback={delivery.kind}>
  <div
    class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-relaxed whitespace-pre-wrap text-text-quiet wrap-anywhere"
  >
    {delivery.words}
  </div>
  <div class={["mt-tight text-note", delivery.kind === "unknown" ? "text-alert" : "text-text-faint"]} role="status">
    {state}
  </div>
</div>

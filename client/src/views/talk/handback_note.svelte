<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- A child session handing its work back to this room (client D86):
the child, linked to the session that did the work, then how it ended,
finished with who verified it or stopped with the reason. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import { called, residentAt } from "./naming";
  import type { HandbackNote } from "../../wire";

  interface Props {
    readonly from: string | null | undefined;
    readonly handback: HandbackNote;
    readonly t: number;
  }

  const { from, handback, t }: Props = $props();

  const u = ui();
  const { lang } = u;
  const child = $derived.by(() => {
    const address = residentAt(from);
    return address === null ? (from ?? "") : called(address, null, $lang);
  });
  const session = $derived("finished" in handback ? handback.finished.session : handback.stopped.session);
</script>

<div class="my-snug flex flex-wrap items-baseline gap-snug text-note text-text-faint" role="note">
  <a href={toFragment({ kind: "run", run: session })} class="text-text-quiet hover:text-text">{child}</a>
  <span>{say($lang, "talk_handback")}</span>
  {#if "finished" in handback}
    <span class="text-accent">{fill(say($lang, "talk_handback_finished"), { by: handback.finished.verified_by })}</span>
  {:else}
    <span class="text-alert">{fill(say($lang, "talk_handback_stopped"), { because: handback.stopped.because })}</span>
  {/if}
  <span>· {clock($lang, t)}</span>
</div>

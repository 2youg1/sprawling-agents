<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- What else a turn came to beside what it said: words that arrived
from the User or a resident, a child's handback, a wait on the User or
on another room's reply, a refusal, messages the run let go, and a
record that did not read back (client D86). `checkpointed` draws nothing:
a commit the run checkpointed is a fact for the run page, and the thread's
question is what this turn did or waits on. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import HandbackNote from "./handback_note.svelte";
  import LetterNote from "./letter_note.svelte";
  import Person from "./person.svelte";
  import ReplyWaitNote from "./reply_wait_note.svelte";
  import RefusedNote from "./refused_note.svelte";
  import type { Note } from "../../wire";

  interface Props {
    readonly note: Note;
  }

  const { note }: Props = $props();

  const u = ui();
  const { lang } = u;
</script>

{#if "arrived" in note}
  {#if note.arrived.handback !== undefined && note.arrived.handback !== null}
    <HandbackNote from={note.arrived.from} handback={note.arrived.handback} session={note.arrived.session ?? null} t={note.arrived.t} />
  {:else if note.arrived.by === "resident"}
    <LetterNote
      from={note.arrived.from}
      said={note.arrived.said ?? ""}
      kind={note.arrived.kind ?? null}
      session={note.arrived.session ?? null}
      t={note.arrived.t}
    />
  {:else}
    <Person
      text={note.arrived.said ?? ""}
      label={say($lang, "talk_you")}
      at={note.arrived.t}
    />
  {/if}
{:else if "awaiting_reply" in note}
  <ReplyWaitNote on={note.awaiting_reply.on} until={note.awaiting_reply.until} ended={note.awaiting_reply.ended} />
{:else if "refused" in note}
  <RefusedNote error={note.refused.error} />
{:else if "waiting" in note}
  <div class="my-snug text-note text-alert">{say($lang, "talk_waiting_you")}</div>
{:else if "discarded" in note}
  <div class="my-snug text-note text-text-faint">
    {fill(say($lang, "talk_discarded"), { n: String(note.discarded.count) })}
  </div>
{:else if "unreadable" in note}
  <!-- A record that did not read back stays in the turn with what
       stopped the reading, and the page offers the Ledger, where the
       record itself can still be read. -->
  <div class="my-snug rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet">
    <span class="text-alert">{fill(say($lang, "talk_unreadable"), { at: String(note.unreadable.at) })}</span>
    · {note.unreadable.cause}
    <div class="mt-tight">
      <a href={toFragment({ kind: "record", lens: "ledger" })} class="text-text-faint hover:text-text-quiet">
        {say($lang, "talk_unreadable_read")}
      </a>
    </div>
  </div>
{/if}

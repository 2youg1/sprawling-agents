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
  import Card from "./refusal_card.look.svelte";
  import HandbackNote from "./handback_note.svelte";
  import LetterNote from "./letter_note.svelte";
  import NoteRow from "./note.look.svelte";
  import NotePlace from "./note_place.svelte";
  import Person from "./person.svelte";
  import ReplyWaitNote from "./reply_wait_note.svelte";
  import RefusedNote from "./refused_note.svelte";
  import { unreadableLook } from "./refused";
  import type { ForkEntry, ForkPlan } from "./forking";
  import type { Note, RunId, Turn } from "../../wire";

  interface Props {
    readonly note: Note;
    readonly turn: Turn;
    readonly run: RunId;
    readonly onFork?: ((plan: ForkPlan) => void) | undefined;
    readonly onHover: (entry: ForkEntry | null) => void;
  }

  const { note, turn, run, onFork, onHover }: Props = $props();

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
      entry={{ kind: "message", turn, text: note.arrived.said ?? "" }}
      {run}
      {onFork}
      {onHover}
    />
  {/if}
{:else if "awaiting_reply" in note}
  <ReplyWaitNote on={note.awaiting_reply.on} until={note.awaiting_reply.until} ended={note.awaiting_reply.ended} />
{:else if "refused" in note}
  <RefusedNote error={note.refused.error} />
{:else if "waiting" in note}
  <NotePlace rhythm="line">
    <NoteRow role={undefined} pieces={[{ kind: "words", text: say($lang, "talk_waiting_you"), ink: "alert" }]} />
  </NotePlace>
{:else if "discarded" in note}
  <NotePlace rhythm="line">
    <NoteRow
      role={undefined}
      pieces={[{ kind: "words", text: fill(say($lang, "talk_discarded"), { n: String(note.discarded.count) }), ink: "faint" }]}
    />
  </NotePlace>
{:else if "unreadable" in note}
  <NotePlace rhythm="shape">
    <Card {...unreadableLook($lang, note.unreadable.at, note.unreadable.cause)} />
  </NotePlace>
{/if}

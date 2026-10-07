<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- What a resident said to this room, drawn as a letter rather than
as the User's bubble (client D86): left-aligned, unfilled, with an
accent edge, and headed by the kind it was sent as, who sent it, linked
to the sender's room, the session that sent it, and when it arrived
(wire D43, client D90). A letter whose sending was not paired says
"letter" and links the room only, rather than guess either. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import type { RunId } from "../../wire";
  import { kindSaid } from "./inbox";
  import Look from "./letter_note.look.svelte";
  import { called, residentAt } from "./naming";
  import NotePlace from "./note_place.svelte";
  import { named, stamped } from "./note";
  import type { Piece } from "./note";

  interface Props {
    readonly from: string | null | undefined;
    readonly said: string;
    readonly kind: string | null;
    readonly session: RunId | null;
    readonly t: number;
  }

  const { from, said, kind, session, t }: Props = $props();

  const u = ui();
  const { lang } = u;
  const sender = $derived(residentAt(from));
  const head = $derived.by((): Piece[] => [
    { kind: "words", text: kind === null ? say($lang, "talk_letter") : kindSaid($lang, kind), ink: "faint" },
    ...(sender !== null
      ? [named(called(sender, null, $lang), toFragment({ kind: "talk", address: sender }))]
      : from !== undefined && from !== null
        ? [named(from, null)]
        : []),
    ...(session === null ? [] : [named(say($lang, "talk_letter_session"), toFragment({ kind: "run", run: session }))]),
    stamped(clock($lang, t)),
  ]);
</script>

<NotePlace rhythm="shape">
  <Look label={say($lang, "talk_letter")} {head} {said} />
</NotePlace>

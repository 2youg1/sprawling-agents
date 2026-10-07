<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- A child session handing its work back to this room (client D86):
the child, linked to the session that did the work (`Arrived.session`,
wire D43; the child's room when the sending was not paired), then how
it ended, finished with who verified it or stopped with the reason. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { toFragment } from "../../core/route";
  import { clock } from "../../core/time";
  import { called, residentAt } from "./naming";
  import NoteRow from "./note.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { handbackLook, named } from "./note";
  import type { HandbackNote, RunId } from "../../wire";

  interface Props {
    readonly from: string | null | undefined;
    readonly handback: HandbackNote;
    readonly session: RunId | null;
    readonly t: number;
  }

  const { from, handback, session, t }: Props = $props();

  const u = ui();
  const { lang } = u;
  const address = $derived(residentAt(from));
  const child = $derived(address === null ? (from ?? "") : called(address, null, $lang));
  const link = $derived(
    session !== null ? toFragment({ kind: "run", run: session }) : address !== null ? toFragment({ kind: "talk", address }) : null,
  );
</script>

<NotePlace rhythm="line">
  <NoteRow {...handbackLook($lang, named(child, link), handback, clock($lang, t))} />
</NotePlace>

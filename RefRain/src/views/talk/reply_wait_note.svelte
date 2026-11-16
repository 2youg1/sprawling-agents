<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { ui } from "../../ui";
  import { toFragment } from "../../core/route";
  import { called } from "./naming";
  import NoteRow from "./note.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { named, replyWaitLook } from "./note";
  import { ticker } from "./timing";
  import type { Address, ReplyEnded } from "../../wire";

  interface Props {
    readonly on: Address;
    readonly until: number;
    readonly ended: ReplyEnded | null | undefined;
  }

  const { on, until, ended }: Props = $props();

  const u = ui();
  const { lang } = u;
  const clock = ticker(u.now);
  const room = $derived(named(called(on, null, $lang), toFragment({ kind: "talk", address: on })));
</script>

<NotePlace rhythm="line">
  <NoteRow {...replyWaitLook($lang, room, until - $clock, ended)} />
</NotePlace>

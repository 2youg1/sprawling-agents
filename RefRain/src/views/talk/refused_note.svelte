<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One refusal a turn came to, said by `refusedLook` and drawn as the
  // thread's refusal card.
  import { ui } from "../../ui";
  import Card from "./refusal_card.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { refusedLook } from "./refused";
  import type { AxError } from "../../wire";

  const { error }: { readonly error: AxError } = $props();
  const { lang } = ui();
</script>

<NotePlace rhythm="shape">
  <Card {...refusedLook($lang, error)} />
</NotePlace>

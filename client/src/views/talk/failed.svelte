<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // A message that got no reply, drawn where the reply would have been,
  // as the thread's refusal card (`failedLook`): nothing a person sent
  // may end in silence.
  import { ui } from "../../ui";
  import type { AxError } from "../../wire";
  import Card from "./refusal_card.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { failedLook } from "./refused";

  interface Props {
    // What happened, already in the reader's language.
    readonly what: string;
    // The city's refusal, when there is one to show.
    readonly error?: AxError | undefined;
    // Whether the way out goes through the model settings; absent, the
    // refusal decides.
    readonly settings?: "offered" | "absent";
    readonly onRetry?: (() => void) | undefined;
  }

  const { what, error, settings, onRetry }: Props = $props();
  const { lang } = ui();
</script>

<NotePlace rhythm="shape">
  <Card {...failedLook($lang, what, error, { settings: settings ?? "decided", onRetry })} />
</NotePlace>

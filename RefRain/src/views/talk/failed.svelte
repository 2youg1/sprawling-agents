<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { ui } from "../../ui";
  import type { AxError } from "../../wire";
  import Card from "./refusal_card.look.svelte";
  import NotePlace from "./note_place.svelte";
  import { failedLook } from "./refused";

  interface Props {
    readonly what: string;
    readonly error?: AxError | undefined;
    readonly settings?: "offered" | "absent" | undefined;
    readonly onRetry?: (() => void) | undefined;
  }

  const { what, error, settings, onRetry }: Props = $props();
  const { lang } = ui();
</script>

<NotePlace rhythm="shape">
  <Card {...failedLook($lang, what, error, { settings: settings ?? "decided", onRetry })} />
</NotePlace>

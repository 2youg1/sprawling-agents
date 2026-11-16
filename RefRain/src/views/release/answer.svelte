<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { ui } from "../../ui";
  import type { ReleaseAnswer } from "../../wire";
  import { lookOf } from "./answer";
  import Look from "./answer.look.svelte";

  interface Props {
    readonly answer: ReleaseAnswer;
  }

  const { answer }: Props = $props();
  const { lang } = ui();

  let selection = $state<{
    readonly answer: ReleaseAnswer | null;
    readonly command: string | null;
  }>({ answer: null, command: null });

  const look = $derived(
    lookOf(answer, $lang, selection.answer === answer ? selection.command : null, {
      choose: (command) => {
        selection = { answer, command };
      },
    }),
  );
</script>

<Look {...look} />

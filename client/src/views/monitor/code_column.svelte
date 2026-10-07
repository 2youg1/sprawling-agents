<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The seat of the code column (client D95): it reads the person's
// language and editor and hands them, with the run's files, to
// `./code_column.ts`, whose value the look (`code_column.look.svelte`)
// draws.
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import { lookOf } from "./code_column";
  import Look from "./code_column.look.svelte";
  import type { Touched } from "./trace";

  interface Props {
    readonly files: readonly Touched[];
    // Whether the run still takes steers; a finished run refuses them.
    readonly live: boolean;
    // Puts a steer in front of the person to finish and send.
    readonly onDraft: (text: string) => void;
    // Sends a steer to the run as it stands.
    readonly onSteer: (text: string) => void;
  }

  const { files, live, onDraft, onSteer }: Props = $props();
  const { lang, prefs } = ui();

  const look = $derived(lookOf({ files, live, editor: prefs.editor() }, $lang, { draft: onDraft, steer: onSteer }));
</script>

<Look {...look} />

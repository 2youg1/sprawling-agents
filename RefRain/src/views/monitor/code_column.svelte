<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors
</script>

<script lang="ts">
  import { ui } from "../../ui";
  import { lookOf } from "./code_column";
  import Look from "./code_column.look.svelte";
  import type { Touched } from "./trace";

  interface Props {
    readonly files: readonly Touched[];
    readonly live: boolean;
    readonly onDraft: (text: string) => void;
    readonly onSteer: (text: string) => void;
  }

  const { files, live, onDraft, onSteer }: Props = $props();
  const { lang, prefs } = ui();

  const look = $derived(
    lookOf({ files, live, editor: prefs.editor() }, $lang, { draft: onDraft, steer: onSteer }),
  );
</script>

<Look {...look} />

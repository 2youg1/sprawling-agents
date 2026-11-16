<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Draft } from "./draft";
  import type { SetDraft } from "./advanced";

  export interface AdvancedProps {
    readonly draft: Draft;
    readonly setDraft: SetDraft;
    readonly onRenamed: () => void;
  }
</script>

<script lang="ts">
  import { HALL } from "../../shared/buildings";
  import { ui } from "../../../ui";
  import { lookOf } from "./advanced";
  import Look from "./advanced.look.svelte";

  const { draft, setDraft, onRenamed }: AdvancedProps = $props();

  const u = ui();
  const { lang } = u;
  const config = u.conn.asking.ask({ config: { addr: HALL } });
  const defaults = $derived.by(() => {
    const answer = $config;
    return answer !== undefined && "config" in answer ? answer.config.tuning : undefined;
  });

  const look = $derived(lookOf(draft, defaults, $lang, { setDraft, onRenamed }));
</script>

<Look {...look} />

<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Look from "./empty.look.svelte";
  import { lookOf } from "./empty";
  import type { EmptyStateProps } from "./empty";

  const props: EmptyStateProps = $props();
  const { lang } = ui();
  const look = $derived(lookOf(props, say($lang, props.missing)));
</script>

<Look {...look} />

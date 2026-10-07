<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of one timeline row: it lends the decision (`./entry`) the
  // person's language and draws whatever `./entry.look.svelte` is. The
  // timeline owns which record is open; this row only reports a press.

  import { ui } from "../../ui";
  import { lookOf } from "./entry";
  import type { EntryProps } from "./entry";
  import Look from "./entry.look.svelte";

  const props: EntryProps = $props();
  const lang = ui().lang;
  const look = $derived(lookOf(props, $lang));
</script>

<Look {...look} />

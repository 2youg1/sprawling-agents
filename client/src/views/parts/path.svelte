<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of one printed path: callers import this file, and it lends
  // `./path.ts` the page's language and the city's `reveal` command, then
  // draws whatever `./path.look.svelte` is with the value that builds.
  import { reveal } from "../../core/commands";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { lookOf, type PathProps } from "./path";
  import Look from "./path.look.svelte";

  const props: PathProps = $props();
  const { lang, send } = ui();

  const look = $derived(
    lookOf(props, { reveal: say($lang, "path_reveal"), inert: say($lang, "path_reveal_inert") }, (address) => {
      send(reveal(address));
    }),
  );
</script>

<Look {...look} />

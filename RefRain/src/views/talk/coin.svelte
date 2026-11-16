<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of the coin key (docs/frontend-method.md §7I, client D18):
  // it reads the language and hands `./coin.ts`'s value to whatever
  // `./coin.look.svelte` is.
  import type { Sending } from "../../core/doing";
  import { ui } from "../../ui";
  import { lookOf } from "./coin";
  import type { Face } from "./coin_face";
  import Look from "./coin.look.svelte";

  interface Props {
    readonly face: Face;
    // Where a send lands: `/dispatch`, `/steer`, or a steer after the tool
    // call (client/Spec.lean §4-13), said by the send face's name.
    readonly sending: Sending;
    readonly onStop: () => void;
  }

  const { face, sending, onStop }: Props = $props();
  const { lang } = ui();
</script>

<Look {...lookOf(face, sending, $lang, onStop)} />

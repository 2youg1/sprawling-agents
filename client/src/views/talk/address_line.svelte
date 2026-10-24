<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The conversation's address line (client/Spec.lean 4-10a): where main
  // stands, said once, and the way into the command palette that a touch
  // screen without Accel-`/` has. The session's number is the city's
  // (`Query::Sessions`, the same question the page's workspace asks, which
  // `asking` merges), so a reload draws the same number; the palette it
  // opens is the shell's one palette (`palette_door.svelte.ts`), and
  // closing it gives the focus back to this line.
  import { lineIn } from "../../core/stretches";
  import { ui } from "../../ui";
  import type { Address, Seq } from "../../wire";
  import { openPalette } from "../palette_door.svelte";
  import { addressLineOf } from "./address_line";
  import Look from "./address_line.look.svelte";

  interface Props {
    readonly address: Address;
    // The earlier session main shows; absent is the room's current one.
    readonly session?: Seq | undefined;
    // What a press does; the shell's palette unless a fixture says.
    readonly onOpen?: (() => void) | undefined;
  }

  const { address, session, onOpen = openPalette }: Props = $props();

  const u = ui();
  const { lang } = u;
  const lines = $derived(u.conn.asking.ask({ sessions: { room: address } }));
  const began = $derived(session ?? lineIn($lines !== undefined && "sessions" in $lines ? $lines.sessions : undefined, undefined)?.began ?? null);
</script>

<Look {...addressLineOf($lang, address, began, onOpen)} />

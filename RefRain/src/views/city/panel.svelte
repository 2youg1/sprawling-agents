<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The seat of what was picked on the drawing or the table: it reads
  // the belief and the language and hands `./panel.ts`'s value to
  // whatever `./panel.look.svelte` is. The panel is the third column on
  // a wide screen, a drawer over the drawing on a medium one, and a
  // block under the drawing on a narrow one - one component either way,
  // so the three widths cannot drift apart.
  import { heldWithin } from "../../core/belief/rooms";
  import { ui } from "../../ui";
  import type { Address, CityAnswer } from "../../wire";
  import { panelLookOf } from "./panel";
  import Look from "./panel.look.svelte";

  interface PanelProps {
    readonly addr: Address;
    readonly city: CityAnswer;
    readonly onClose: () => void;
  }

  const { addr, city, onClose }: PanelProps = $props();

  const u = ui();
  const { lang } = u;
  const belief = u.conn.belief;

  const look = $derived(
    panelLookOf({ addr, city, held: heldWithin($belief, addr), lang: $lang }, () => {
      onClose();
    }),
  );
</script>

<Look {...look} />

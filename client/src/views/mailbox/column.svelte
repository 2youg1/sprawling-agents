<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The seat of the mailbox column (client/Spec.lean §4-49): its head -
  // the name, the link when it is not live, and on a phone the way back -
  // then deciding, working, recent and the notices, in the order of what
  // needs the person, drawn by whatever `./column.look.svelte` is.
  // `mailbox.svelte` seats it in the layer under the keys; `#/gallery`
  // seats it in a frame of the same width.
  import type { Snippet } from "svelte";
  import { createAttachmentKey } from "svelte/attachments";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { recover } from "../notice_recovery";
  import { linkOf } from "./column";
  import Look from "./column.look.svelte";
  import Deciding from "./deciding.svelte";
  import { walk } from "./entries";
  import Notices from "./notices.svelte";
  import Recent from "./recent.svelte";
  import Working from "./working.svelte";

  interface Props {
    // Puts the mailbox away: after following a row, and from the way
    // back on a phone.
    readonly onClose: () => void;
  }

  const { onClose }: Props = $props();

  const u = ui();
  const { lang } = u;
  const link = u.conn.state;

  let scroller = $state<HTMLElement | undefined>(undefined);

  // One scroller for every section: j, k and the digits are heard on it,
  // below every entry (client/Spec.lean §7-11).
  const HOLD = createAttachmentKey();
  const hold = (node: HTMLElement): (() => void) => {
    scroller = node;
    const heard = (event: KeyboardEvent): void => {
      walk(node, event);
    };
    node.addEventListener("keydown", heard);
    return () => {
      node.removeEventListener("keydown", heard);
      if (scroller === node) scroller = undefined;
    };
  };

  const body: Snippet = sections;
</script>

{#snippet sections()}
  <Deciding onLeave={onClose} />
  <Working onLeave={onClose} />
  <Recent scroller={() => scroller} onLeave={onClose} />
  <Notices />
{/snippet}

<Look
  title={say($lang, "edge_mailbox")}
  back={{ label: say($lang, "mailbox_back"), onPress: onClose }}
  link={linkOf($link, $lang, (lever, error) => {
    recover(u, lever, { error, about: null });
  })}
  scroller={{ [HOLD]: hold }}
  {body}
/>

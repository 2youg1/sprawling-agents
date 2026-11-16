<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->
<script lang="ts">
  import { ui } from "../../ui";
  import type { Address, FindAnswer } from "../../wire";
  import { openDocument } from "../inspect/open.svelte";
  import { searchLookOf } from "./search";
  import Look from "./search.look.svelte";

  interface Props {
    readonly under: Address;

    readonly titleId: string;
    readonly onClose: () => void;
  }

  const { under, titleId, onClose }: Props = $props();

  const uid = $props.id();
  const u = ui();
  const { lang } = u;

  let text = $state("");
  let cursor = $state(0);

  const asked = $derived(u.conn.asking.ask({ find: { under, text: text.trim() } }));

  let found = $state.raw<FindAnswer | undefined>(undefined);
  $effect(() => {
    const answer = $asked;
    if (answer !== undefined && "find" in answer) found = answer.find;
  });

  const look = $derived(
    searchLookOf(
      { under, titleId, uid, text, cursor, found, lang: $lang },
      {
        type: (typed) => {
          text = typed;
          cursor = 0;
        },
        point: (at) => {
          cursor = at;
        },
        open: (path) => {
          openDocument({ building: under, path, version: null });
          onClose();
        },
      },
    ),
  );
</script>

<Look {...look} />

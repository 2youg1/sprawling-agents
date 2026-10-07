<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // What the file finder holds (client/Spec.lean §4-62): the building it looks
  // in, the box, the files the city found, and what the city could not
  // say. An APG Combobox: the box keeps the focus, ↓/↑ move the active
  // option, Enter opens it in the right side through `openDocument` at
  // the worktree's text. The page never claims that a directory it was
  // not told about holds no such name, so an answer the city cut short
  // says so under the list (`Query::Find`, `crates/wire/Spec.lean`
  // §8-82). The modal around it is `views/finder.svelte`'s, and opening
  // it puts the focus in the box, the first control the dialog holds.
  //
  // This is the seat: it holds the text, the cursor and the question,
  // and hands `./search.ts`'s value to whatever `./search.look.svelte` is.

  import { ui } from "../../ui";
  import type { Address, FindAnswer } from "../../wire";
  import { openDocument } from "../inspect/open.svelte";
  import { searchLookOf } from "./search";
  import Look from "./search.look.svelte";

  interface Props {
    // The building looked in, named in the title.
    readonly under: Address;
    // The id of the title, which names the dialog around this too.
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
  // The last answer stays drawn while the next one is asked, so the list
  // does not blink empty on every key.
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

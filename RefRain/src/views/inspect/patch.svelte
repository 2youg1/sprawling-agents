<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { toFragment } from "../../core/route";
  import { ui } from "../../ui";
  import type { Address, HunksAnswer } from "../../wire";
  import { quoteLine } from "../changes";
  import type { CodeLine } from "../changes";
  import { quoteInto } from "../talk/quoting";
  import { lookOf, type PatchLook } from "./patch";
  import Look from "./patch.look.svelte";

  interface Props {
    readonly patch: HunksAnswer;

    readonly talk?: Address | undefined;

    readonly cursor?: number | null;
    readonly onCursor?: (line: CodeLine) => void;
  }

  const { patch, talk, cursor = null, onCursor }: Props = $props();

  const u = ui();
  const lang = u.lang;

  function choose(line: CodeLine): void {
    if (talk === undefined) return;
    quoteInto(u.prefs, talk, quoteLine(patch, line));
    onCursor?.(line);
  }

  const look: PatchLook = $derived(
    lookOf(
      patch,
      cursor,
      {
        withheld: (n, reason) => fill(say($lang, "run_withheld"), { n, reason }),
        folded: (n) => fill(say($lang, "change_folded"), { n }),
        quote: (n) => fill(say($lang, "change_line_quote"), { n }),
      },
      {
        href: talk === undefined ? undefined : toFragment({ kind: "talk", address: talk }),
        choose,
      },
    ),
  );
</script>

<Look {...look} />

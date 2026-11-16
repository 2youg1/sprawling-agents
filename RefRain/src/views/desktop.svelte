<script lang="ts" module>
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors
</script>

<script lang="ts">
  import { untrack } from "svelte";

  import { configureDesktop } from "../core/commands";
  import { readDocument } from "../core/document";
  import { ui } from "../ui";
  import type { Address, Query } from "../wire";
  import { desktopScopeAt, lookOf } from "./desktop";
  import Look from "./desktop.look.svelte";

  interface Props {
    readonly addr: Address;
  }

  const { addr }: Props = $props();

  const u = ui();
  const lang = u.lang;

  let draft = $state("");
  let edited = $state(false);

  const question = $derived<Query>({ document: { at: desktopScopeAt(addr) } });
  const held = $derived(u.conn.asking.ask(question));
  const read = $derived(readDocument($held));
  const onDisk = $derived(read.kind === "held" ? read.value.text : "");

  let was: Address | undefined = undefined;
  $effect(() => {
    const at = addr;
    const text = onDisk;
    if (!untrack(() => edited) || was !== at) {
      edited = false;
      draft = text;
    }
    was = at;
  });

  function save(): void {
    if (u.send(configureDesktop(addr, draft))) {
      edited = false;
    }
  }

  const file = $derived.by((): "missing" | "empty" | "held" => {
    if (read.kind === "unavailable") return "missing";
    return read.kind === "held" && onDisk === "" ? "empty" : "held";
  });

  const look = $derived(
    lookOf({ addr, draft, edited, file }, $lang, {
      type: (text) => {
        draft = text;
        edited = true;
      },
      save,
    }),
  );
</script>

<Look {...look} />

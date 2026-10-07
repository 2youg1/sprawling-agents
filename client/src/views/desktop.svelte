<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The seat of a building's desktop allowlist editor (client D95): it
// asks for the file, holds the draft, and sends the save; `./desktop.ts`
// turns them into the value the look (`desktop.look.svelte`) draws.
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
  // The city answers `unavailable` both for a building with no allowlist
  // yet and for a file it could not read, so the box starts empty for
  // either and the page says what is missing and how a save creates it,
  // rather than calling the allowlist empty or printing the question the
  // city could not answer: a save writes the file whole either way, and
  // the server reads an unreadable file as a closed door.
  const read = $derived(readDocument($held));
  const onDisk = $derived(read.kind === "held" ? read.value.text : "");

  // The box follows the file until somebody types in it, and follows it
  // again once their text has landed. A draft that outlived its save
  // would show a person their own words beside a file that no longer
  // says them. Whether somebody has typed is read without tracking: a
  // save clearing that flag must not pull the old file back into the
  // box while the fresh answer is still on its way.
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

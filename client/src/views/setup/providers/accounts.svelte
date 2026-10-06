<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The seat of one endpoint's account editor: it holds the editor's
  // state, lends the wiring (`./account_editor`, `./accounts`) the page's doors - the
  // socket, the vault route, the language - and the two elements focus
  // returns to, and draws whatever `./accounts.look.svelte` is. What a
  // press does is decided in the wiring and nowhere here (client D92).
  //
  // The city's refusal of a list this editor sent is this editor's to
  // show, beside the list, for the same reason the attach form keeps
  // its own: the corner is for refusals no open page is responsible for.

  import type { EndpointSummary } from "../../../wire";

  export interface AccountsProps {
    readonly endpoint: EndpointSummary;
  }
</script>

<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { enrol } from "../../../core/enrol";
  import { ui } from "../../../ui";
  import { freshEditor, leave, refuse, settle } from "./account_editor";
  import type { Editor, Hands, Step } from "./account_editor";
  import { lookOf } from "./accounts";
  import type { Holds } from "./accounts";
  import Look from "./accounts.look.svelte";

  const { endpoint }: AccountsProps = $props();
  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  const editor: Editor = $state(freshEditor());
  let heading: HTMLElement | undefined;
  // Two registries no draw reads, so plain records rather than
  // reactive maps: a `SvelteMap` read inside the derived look and
  // written by the attachment it hands out would be a write during a
  // derivation.
  const held: Record<string, HTMLElement | undefined> = {};
  const attachments: Record<string, Attachment<HTMLElement> | undefined> = {};

  const hands: Hands = {
    endpoint: () => endpoint,
    lang: () => $lang,
    reach: () => ({ origin: u.origin, token: u.pairing }),
    send: (command) => u.send(command),
    enrol,
    follow: (id, step) => {
      void tick().then(() => held[`${id}:${step}`]?.querySelector("button")?.focus());
    },
    focusHeading: () => {
      void tick().then(() => heading?.focus());
    },
  };

  // One attachment per control for the life of the seat, so a redraw
  // does not let go of an element and take it again.
  const holds: Holds = {
    heading: (node) => {
      heading = node;
      return () => {
        heading = undefined;
      };
    },
    control: (id: string, step: Step) => {
      const key = `${id}:${step}`;
      const kept = attachments[key];
      if (kept !== undefined) return kept;
      const made: Attachment<HTMLElement> = (node) => {
        held[key] = node;
        return () => {
          if (held[key] === node) held[key] = undefined;
        };
      };
      attachments[key] = made;
      return made;
    },
  };

  $effect(() => {
    settle(editor, endpoint);
  });
  $effect(() => {
    const refused = $belief.refusal;
    if (refused === null || editor.sent === null) return;
    refuse(editor, refused);
    u.conn.dismissRefusal();
  });
  onDestroy(() => {
    leave(editor);
  });

  const look = $derived(lookOf(editor, hands, holds));
</script>

<Look {...look} />

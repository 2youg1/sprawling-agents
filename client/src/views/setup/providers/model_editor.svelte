<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { EndpointsAnswer } from "../../../wire";
  import type { FlightState } from "./flight";

  export interface ModelEditorProps {
    readonly answer: EndpointsAnswer;
    readonly name: string;
    readonly flight: FlightState;
    // The disclosure's initial state; afterwards the native summary owns it.
    readonly open?: boolean;
  }
</script>

<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { get } from "svelte/store";
  import type { Attachment } from "svelte/attachments";

  import { ui } from "../../../ui";
  import { ModelTable } from "../models";
  import type { ModelRow } from "../models";
  import type { TableAbout } from "../model_rows";
  import {
    awaiting,
    choose,
    freshModelEditor,
    leave,
    lookOf,
    refuse,
    settleAnswer,
    settleProbe,
  } from "./model_editor";
  import type { Hands } from "./model_editor";
  import Look from "./model_editor.look.svelte";

  const { answer, name, flight, open }: ModelEditorProps = $props();
  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;
  // svelte-ignore state_referenced_locally (each keyed endpoint row seeds one editor; later answers settle it rather than replacing its draft)
  const editor = $state(freshModelEditor(answer, name, flight));
  // svelte-ignore state_referenced_locally (open is the documented initial state; the native summary owns it afterwards)
  let expanded = $state(open ?? false);
  let summary: HTMLElement | undefined;

  const hands: Hands = $derived.by((): Hands => ({
    answer: () => answer,
    probed: () => get(belief).probed,
    send: (command) => u.send(command),
  }));
  const about = $derived.by((): TableAbout | null => {
    const endpoint = answer.endpoints.find((each) => each.name === name);
    return endpoint === undefined
      ? null
      : { endpoint: endpoint.name, baseUrl: endpoint.base_url, face: endpoint.dialect };
  });
  const look = $derived(lookOf(editor, hands, $lang));
  const holdSummary: Attachment<HTMLElement> = (node) => {
    summary = node;
    return () => {
      if (summary === node) summary = undefined;
    };
  };

  $effect(() => {
    settleProbe(editor, $belief.probed);
  });
  $effect(() => {
    settleAnswer(editor, answer);
  });
  $effect(() => {
    const refused = $belief.refusal;
    if (refused === null || !awaiting(editor)) return;
    refuse(editor, refused);
    u.conn.dismissRefusal();
  });
  $effect(() => {
    if (!editor.focusSummary) return;
    editor.focusSummary = false;
    void tick().then(() => summary?.focus());
  });
  onDestroy(() => {
    leave(editor);
  });
</script>

{#snippet table()}
  <ModelTable
    served={editor.served}
    {about}
    state={editor.table}
    onChosen={(rows: readonly ModelRow[]) => {
      choose(editor, rows);
    }}
  />
{/snippet}

<Look
  {...look}
  {table}
  {holdSummary}
  open={expanded}
  onToggle={(next) => {
    expanded = next;
  }}
/>

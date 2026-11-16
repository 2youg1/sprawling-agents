<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The editor itself: the CodeMirror view `editing.ts` builds, mounted
  // once per document with the baseline's editor text and a kept draft's
  // changes on it, and handed to the session. Imported lazily by
  // `document.svelte`, so the editor's chunk is fetched with the first
  // document a person opens.
  import { untrack } from "svelte";

  import { editorWire } from "./editor";
  import Editor from "./editor.look.svelte";
  import { openEditor } from "./editing";
  import type { Session } from "./session.svelte";

  interface Props {
    readonly session: Session;
    readonly label: string;
    readonly phrases: Readonly<Record<string, string>>;
  }

  const { session, label, phrases }: Props = $props();

  let host = $state<HTMLDivElement>();
  const wire = editorWire((box) => {
    host = box;
  });

  // The view is built once the box is drawn, from the opening the
  // session held then; nothing it reads afterwards rebuilds it, because a
  // rebuilt view would drop the cursor, the undo history and a
  // composition in progress.
  $effect(() => {
    const parent = host;
    if (parent === undefined) return undefined;
    return untrack(() => {
      const opening = session.opening;
      if (opening === null) return undefined;
      const editing = openEditor({
        parent,
        text: opening.text,
        changes: opening.changes,
        label,
        phrases,
        onEdit: () => {
          session.edited();
        },
        onSave: () => {
          session.save();
        },
      });
      session.attach(editing);
      return () => {
        editing.destroy();
      };
    });
  });

  $effect(() => {
    session.editing?.readOnly(session.locked !== null);
  });
</script>

<Editor {wire} />

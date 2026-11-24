// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, onSettled } from "solid-js";
import { storeValue } from "../../core/solid_store";
import EditorHost from "./editor_host";
import { openEditor } from "./editing";
import type { Session } from "./session";

export default function Source(props: {
  readonly session: Session;
  readonly label: string;
  readonly phrases: Readonly<Record<string, string>>;
  readonly diagramLabel?: string | undefined;
}) {
  const state = storeValue(props.session.state);
  let host: HTMLDivElement | undefined;
  onSettled(() => {
    const opening = state().opening;
    if (host === undefined || opening === null) return;
    const editor = openEditor({
      parent: host,
      text: opening.text,
      changes: opening.changes,
      label: props.label,
      phrases: props.phrases,
      markdown: props.diagramLabel === undefined ? undefined : { diagramLabel: props.diagramLabel },
      onEdit: () => props.session.edited(),
      onSave: () => props.session.save(),
    });
    props.session.attach(editor);
    return () => editor.destroy();
  });
  const current = createMemo(() => state().editing);
  const locked = createMemo(() => state().locked !== null);
  createEffect(
    () => [current(), locked()] as const,
    ([editor, on]) => {
      editor?.readOnly(on);
    },
  );
  return (
    <EditorHost
      ref={(node) => {
        host = node;
      }}
    />
  );
}

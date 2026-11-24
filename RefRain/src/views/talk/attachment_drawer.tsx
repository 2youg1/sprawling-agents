// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createSignal, For, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { citationOf, openCitation } from "./attachments";
import type { DraftAttachment } from "./attachments";

interface PreviewProps {
  readonly attachment: DraftAttachment;
  readonly lang: Lang;
  readonly remove: (id: string) => void;
}

function AttachmentPreview(props: PreviewProps) {
  const [url, setUrl] = createSignal<string | undefined>();
  createEffect(
    () => props.attachment.selection,
    (selection) => {
      if (selection.kind !== "pdf-region") {
        setUrl(undefined);
        return;
      }
      const preview = URL.createObjectURL(selection.image);
      setUrl(preview);
      return () => {
        URL.revokeObjectURL(preview);
      };
    },
  );
  const excerpt = () => {
    const selection = props.attachment.selection;
    return selection.kind === "pdf-region" ? undefined : selection.text;
  };
  return (
    <li class="flex min-w-0 flex-col gap-tight pb-snug">
      <div class="flex min-w-0 items-center gap-base">
        <button
          data-control="surface"
          type="button"
          class="min-w-0 truncate text-text underline"
          onClick={() => openCitation(citationOf(props.attachment))}
        >
          {props.attachment.label}
        </button>
        <button
          data-control="surface"
          type="button"
          class="shrink-0 text-text-quiet"
          aria-label={fill(say(props.lang, "talk_remove_attachment"), {
            file: props.attachment.label,
          })}
          onClick={() => props.remove(props.attachment.id)}
        >
          {say(props.lang, "talk_remove_attachment_short")}
        </button>
      </div>
      <Show when={url()}>
        {(preview) => (
          <img
            class="max-h-output max-w-full self-start object-contain"
            src={preview()}
            alt={say(props.lang, "talk_attachment_region")}
          />
        )}
      </Show>
      <Show when={excerpt()}>
        {(text) => (
          <blockquote class="border-s border-edge-panel ps-snug text-note whitespace-pre-wrap wrap-anywhere">
            {text()}
          </blockquote>
        )}
      </Show>
    </li>
  );
}

export interface DrawerProps {
  readonly attachments: readonly DraftAttachment[];
  readonly storageFailure: boolean;
  readonly lang: Lang;
  readonly remove: (id: string) => void;
}

export function AttachmentDrawer(props: DrawerProps) {
  return (
    <Show when={props.attachments.length > 0}>
      <details class="mb-snug rounded-panel border border-edge-panel p-snug">
        <summary>{say(props.lang, "talk_attachments")}</summary>
        <ul class="max-h-composer overflow-y-auto pt-tight">
          <For each={props.attachments} keyed={(attachment) => attachment.id}>
            {(attachment) => (
              <AttachmentPreview
                attachment={attachment()}
                lang={props.lang}
                remove={props.remove}
              />
            )}
          </For>
        </ul>
        <Show when={props.storageFailure}>
          <p role="status">{say(props.lang, "talk_attachment_storage_failure")}</p>
        </Show>
      </details>
    </Show>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { openCitation, reviewRewrite } from "./attachments";
import type { DocumentCitation, DocumentRewrite } from "./attachments";

export interface GroundedLinksProps {
  readonly citations: readonly DocumentCitation[];
  readonly proposal: DocumentRewrite | null;
  readonly lang: Lang;
}

export function GroundedLinks(props: GroundedLinksProps) {
  return (
    <>
      <Show when={props.citations.length > 0}>
        <ul class="flex flex-wrap gap-snug text-note">
          <For each={props.citations}>
            {(citation) => (
              <li>
                <button
                  data-control="surface"
                  type="button"
                  class="text-text underline"
                  onClick={() => openCitation(citation)}
                >
                  {fill(say(props.lang, "talk_open_citation"), { file: citation.label })}
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <Show when={props.proposal}>
        {(proposal) => (
          <button
            data-control="surface"
            type="button"
            class="text-text underline"
            onClick={() => reviewRewrite(proposal())}
          >
            {fill(say(props.lang, "talk_review_rewrite"), { file: proposal().label })}
          </button>
        )}
      </Show>
    </>
  );
}

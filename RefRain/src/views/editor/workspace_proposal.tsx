// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, createUniqueId, Show } from "solid-js";
import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import Glyph from "../parts/glyph.solid";
import type { DocumentRewrite } from "../talk/attachments";
import { validReplacement } from "./workspace_text";
import type { ProposalBlock } from "./workspace_text";

interface ReviewProps {
  readonly proposal: DocumentRewrite;
  readonly lang: Lang;
  readonly reason: ProposalBlock | null;
  readonly busy: boolean;
  readonly onReveal: () => void;
  readonly onAccept: (replacement: string) => void;
  readonly onReject: () => void;
}

export function WorkspaceProposal(props: ReviewProps) {
  const description = createUniqueId();
  const field = createUniqueId();
  const [editing, setEditing] = createSignal(false);
  const [amended, setAmended] = createSignal<string>();
  const text = () => amended() ?? props.proposal.replacement;
  const malformed = () => !validReplacement(text());
  const blocked = () => props.reason !== null || props.busy || malformed();
  const explanation = () => {
    if (props.reason === "version") return say(props.lang, "proposal_stale_why");
    if (props.reason === "readonly") return say(props.lang, "refrain_save_locked");
    if (props.reason === "overlap") return say(props.lang, "refrain_proposal_local_overlap");
    if (props.reason === "invalid" || malformed())
      return say(props.lang, "refrain_proposal_invalid");
    return "";
  };
  return (
    <section
      class="workspace-proposal"
      aria-label={say(props.lang, "refrain_proposal_review")}
      aria-busy={props.busy ? "true" : undefined}
    >
      <h3>{say(props.lang, "refrain_proposal_review")}</h3>
      <div class="workspace-proposal-text">
        <div>
          <b>{say(props.lang, "proposal_removed")}</b>
          <pre>{props.proposal.quote}</pre>
        </div>
        <div>
          <b>{say(props.lang, "proposal_added")}</b>
          <Show when={editing()} fallback={<pre>{text()}</pre>}>
            <textarea
              id={field}
              aria-label={say(props.lang, "proposal_amend")}
              value={text()}
              onInput={(event) => setAmended(event.currentTarget.value)}
            />
          </Show>
        </div>
      </div>
      <Show when={explanation() !== ""}>
        <p id={description} role="status">
          {explanation()}
        </p>
      </Show>
      <div class="workspace-proposal-actions">
        <button
          type="button"
          data-control="surface"
          disabled={props.reason === "version" || props.reason === "invalid"}
          onClick={props.onReveal}
        >
          <Glyph name="reveal" size="sm" />
          {say(props.lang, "proposal_show")}
        </button>
        <button
          type="button"
          data-control="surface"
          disabled={blocked()}
          data-control-state={
            props.busy ? "pending" : props.reason !== null ? "refused" : undefined
          }
          aria-describedby={explanation() === "" ? undefined : description}
          onClick={() => props.onAccept(text())}
        >
          <Glyph name={props.busy ? "pulse" : "check"} size="sm" />
          {say(props.lang, amended() === undefined ? "proposal_accept" : "proposal_accept_edited")}
        </button>
        <button
          type="button"
          data-control="surface"
          disabled={props.reason !== null || props.busy}
          aria-expanded={editing() ? "true" : "false"}
          aria-controls={editing() ? field : undefined}
          onClick={() => setEditing(!editing())}
        >
          <Glyph name={editing() ? "chevron" : "propose"} size="sm" />
          {say(props.lang, editing() ? "proposal_back" : "proposal_edit")}
        </button>
        <button type="button" data-control="surface" disabled={props.busy} onClick={props.onReject}>
          <Glyph name="cross" size="sm" />
          {say(props.lang, "proposal_reject")}
        </button>
      </div>
    </section>
  );
}

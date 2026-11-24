// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, For, Show } from "solid-js";
import { say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { storeValue } from "../../core/solid_store";
import { usd } from "../../core/time";
import { ui } from "../../ui";
import type { CommitAnswer, GitOid } from "../../wire";
import { shortOid } from "../changes";
import Copy from "../parts/copy.solid";
import FactLink from "./fact_link.look.solid";
import Whose from "./whose.solid";

interface Props {
  readonly commit: CommitAnswer;
  readonly holds?: (oid: GitOid) => boolean;
  readonly onOpen?: (oid: GitOid) => void;
}

const RUN_SHORT = 8;

export default function CommitFacts(props: Props) {
  const lang = storeValue(ui().lang);
  const [asked, setAsked] = createSignal<GitOid | null>(null);
  const model = () => {
    const { model, effort } = props.commit;
    return effort == null ? model : `${model} · ${effort}`;
  };

  function parent(oid: GitOid): void {
    if (props.holds?.(oid) === true) props.onOpen?.(oid);
    else setAsked(asked() === oid ? null : oid);
  }
  return (
    <div class="flex min-w-0 flex-col gap-base">
      <dl class="grid grid-cols-[repeat(auto-fill,minmax(22ch,1fr))] gap-x-gutter gap-y-base text-note">
        <div class="flex min-w-0 flex-col">
          <dt class="text-text-faint">{say(lang(), "commit_run")}</dt>
          <dd>
            <FactLink
              label={props.commit.run.slice(0, RUN_SHORT)}
              face="figure"
              ink="text"
              wire={{ href: toFragment({ kind: "run", run: props.commit.run, lens: "changes" }) }}
            />
          </dd>
        </div>
        <Show when={props.commit.session}>
          {(session) => (
            <div class="flex min-w-0 flex-col">
              <dt class="text-text-faint">{say(lang(), "commit_session")}</dt>
              <dd class="truncate">
                <FactLink
                  label={session()}
                  face="words"
                  ink="text"
                  wire={{ href: toFragment({ kind: "talk", address: props.commit.actor }) }}
                />
              </dd>
            </div>
          )}
        </Show>
        <Show when={props.commit.model !== ""}>
          <div class="flex min-w-0 flex-col">
            <dt class="text-text-faint">{say(lang(), "commit_model")}</dt>
            <dd class="truncate text-text">{model()}</dd>
          </div>
        </Show>
        <div class="flex min-w-0 flex-col">
          <dt class="text-text-faint">{say(lang(), "commit_spent")}</dt>
          <dd class="figure text-text">{usd(props.commit.spent)}</dd>
        </div>
        <Show when={props.commit.lineage.length > 1}>
          <div class="flex min-w-0 flex-col">
            <dt class="text-text-faint">{say(lang(), "commits_lineage")}</dt>
            <dd class="flex flex-wrap gap-x-snug">
              <For each={props.commit.lineage.slice(1)} keyed>
                {(run) => (
                  <FactLink
                    label={run.slice(0, RUN_SHORT)}
                    face="figure"
                    ink="quiet"
                    wire={{ href: toFragment({ kind: "run", run }) }}
                  />
                )}
              </For>
            </dd>
          </div>
        </Show>
        <div class="flex min-w-0 flex-col">
          <dt class="text-text-faint">{say(lang(), "commit_parents")}</dt>
          <dd class="flex flex-wrap gap-x-snug">
            <For
              each={props.commit.parents ?? []}
              keyed
              fallback={<span class="text-text-faint">{say(lang(), "commit_root")}</span>}
            >
              {(oid) => (
                <FactLink
                  label={shortOid(oid)}
                  face="figure"
                  ink={asked() === oid ? "text" : "quiet"}
                  wire={{
                    type: "button",
                    "aria-expanded": props.holds?.(oid) === true ? undefined : asked() === oid,
                    onclick: () => parent(oid),
                  }}
                />
              )}
            </For>
          </dd>
        </div>
        <div class="col-span-full flex min-w-0 items-center gap-base">
          <dt class="sr-only">{say(lang(), "commit_oid")}</dt>
          <dd class="min-w-0 truncate font-mono text-text-faint">{props.commit.oid}</dd>
          <Copy text={props.commit.oid} />
        </div>
      </dl>
      <Show when={asked()}>
        {(oid) => (
          <div class="border-l border-edge-input pl-base">
            <Whose oid={oid()} />
          </div>
        )}
      </Show>
    </div>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address, CommitAnswer, GitOid } from "../../wire";
import Changes from "../changes.solid";
import CommitFacts from "./commit_facts.solid";
import CommitLine from "./commit_line.solid";
import { disclosureWire } from "./disclosure";
import Disclosure from "./disclosure.look.solid";
import TakeBack from "./take_back.solid";

interface Props {
  readonly building: Address;
  readonly commit: CommitAnswer;
  readonly older: CommitAnswer | null;
  readonly first: boolean;
  readonly open: boolean;
  readonly onToggle: () => void;
  readonly holds: (oid: GitOid) => boolean;
  readonly onOpen: (oid: GitOid) => void;
}

export default function Commit(props: Props) {
  const lang = storeValue(ui().lang);
  return (
    <li class="border-b border-edge" id={`commit-${props.commit.oid}`}>
      <Disclosure
        layout="commit"
        open={props.open}
        wire={disclosureWire(props.open, props.onToggle)}
      >
        <CommitLine commit={props.commit} />
      </Disclosure>
      <Show when={props.open}>
        <div class="flex min-w-0 flex-col gap-base pt-snug pb-wide pl-wide narrow:pl-0">
          <CommitFacts commit={props.commit} holds={props.holds} onOpen={props.onOpen} />
          <Show
            when={props.older}
            fallback={
              <p class="text-note text-text-faint">
                {props.first ? say(lang(), "commits_first") : "…"}
              </p>
            }
          >
            {(older) => (
              <Changes
                base={older().oid}
                head={props.commit.oid}
                talk={props.building}
                act={(path) => (
                  <TakeBack building={props.building} path={path} point={props.commit.oid} />
                )}
              />
            )}
          </Show>
        </div>
      </Show>
    </li>
  );
}

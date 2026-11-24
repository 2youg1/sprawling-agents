// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createMemo } from "solid-js";
import { say } from "../../core/lang";
import { roomOf } from "../../core/route";
import { ui } from "../../ui";
import { isoInstant, moment } from "../../core/time";
import type { CommitAnswer } from "../../wire";
import { shortOid } from "../changes";
interface Props {
  readonly commit: CommitAnswer;
}
import { storeValue } from "../../core/solid_store";

export type PortProps = Props;

export default function BuildingCommitLine(props: Props) {
  const lang = ui().lang;
  const $lang = storeValue(lang);
  const subject = createMemo(() => props.commit.message?.split("\n", 1)[0]?.trim() ?? "");
  return (
    <>
      <span class={"figure text-text-quiet"}>{shortOid(props.commit.oid)}</span>
      {(() => {
        const __rrBranch1 = createMemo(() => {
          const __rrCaptured0 = subject();
          return __rrCaptured0 === ""
            ? { yes: { __rrCaptured0 }, no: undefined }
            : { yes: undefined, no: { __rrCaptured0 } };
        });
        return (
          <Show
            when={__rrBranch1().yes}
            fallback={
              <Show when={__rrBranch1().no}>
                {(__rrActive2) => (
                  <>
                    <span class={"truncate text-text"}>{__rrActive2().__rrCaptured0}</span>
                  </>
                )}
              </Show>
            }
          >
            {(__rrActive2) => (
              <>
                <span class={"truncate text-text-faint"}>{say($lang(), "commits_no_message")}</span>
              </>
            )}
          </Show>
        );
      })()}
      <span class={"truncate font-mono text-text-faint narrow:hidden"}>
        {roomOf(props.commit.actor)}
      </span>
      <time class={"figure text-text-faint narrow:hidden"} datetime={isoInstant(props.commit.at)}>
        {moment(props.commit.at)}
      </time>
    </>
  );
}

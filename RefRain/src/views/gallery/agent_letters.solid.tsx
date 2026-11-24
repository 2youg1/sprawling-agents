// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import Talk from "../talk";
import { SPECIMEN } from "../talk/seat";
import { GalleryCase } from "./case";
import {
  answering,
  FINISHED,
  LEFT,
  LETTERS,
  OLDER,
  QUEUED,
  recordsAt,
  REPLIED,
  SENDS,
  STOPPED,
  TIMED_OUT,
  WAITING,
} from "./agent_letters";
import Stand from "./stand.solid";

export default function AgentLetters() {
  const u = ui();
  const now = u.now();
  const records = recordsAt(now);
  const answers = answering(now);
  const SHOWN: readonly (readonly [string, Address])[] = [
    ["agent messages · the User's steer beside a resident's letter, and a pulled signal", LETTERS],
    ["agent messages · a reply wait in progress", WAITING],
    ["agent messages · a reply wait ended by the reply", REPLIED],
    ["agent messages · a reply wait ended by the timeout", TIMED_OUT],
    ["agent messages · a reply wait ended because the room left", LEFT],
    ["agent messages · a handback, finished", FINISHED],
    ["agent messages · a handback, stopped", STOPPED],
    ["agent messages · two queued signals with their first lines", QUEUED],
    ["agent messages · a letter from an older Ledger, without its kind or session", OLDER],
    ["agent messages · send lines: delivered, queued, knocked a new run, and not recorded", SENDS],
  ];
  return (
    <>
      <For each={SHOWN}>
        {([label, address]) => (
          <>
            <GalleryCase label={label} width={760}>
              <Stand
                link={{ kind: "live", city: "sprawling" }}
                unread={[]}
                waiting={[]}
                answers={answers}
                records={records}
                children={() => (
                  <>
                    <div class="h-[440px] px-wide">
                      <Talk address={address} band={false} seat={SPECIMEN} />
                    </div>
                  </>
                )}
              />
            </GalleryCase>
          </>
        )}
      </For>
    </>
  );
}

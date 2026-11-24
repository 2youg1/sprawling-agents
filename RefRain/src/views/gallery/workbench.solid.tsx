// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { storeValue } from "../../core/solid_store";
import type { Tier } from "../../core/prefs";
import { ui } from "../../ui";
import { SessionPane } from "../shell/panorama";
import Listening from "../talk/listening_view";
import City from "../world/city.solid";
import { Files } from "../world/files_view";
import { SessionSheet } from "../world/sheet_view";
import { OPENING, PICKED, ROOM, TURNS, answering, oid, recordsAt, run } from "./benched";
import { GalleryCase } from "./case";
import ShellSpecimen from "./shell_specimen";
import Stand from "./stand.solid";
interface Shown {
  readonly label: string;
  readonly tier: Tier;
  readonly panel: boolean;
  readonly width: number;
}

const SHOWN: readonly Shown[] = [
  {
    label: "workbench · panorama, a building room with a commit picked",
    tier: "panorama",
    panel: false,
    width: 1440,
  },
  { label: "workbench · panorama at 1920", tier: "panorama", panel: false, width: 1920 },
  {
    label: "workbench · blend, beside the conversation",
    tier: "blend",
    panel: false,
    width: 1440,
  },
  {
    label: "workbench · panorama, the right pane open",
    tier: "panorama",
    panel: true,
    width: 1440,
  },
];

export default function WorkbenchCases() {
  const app = ui();
  const lang = storeValue(app.lang);
  const records = recordsAt(app.now());
  const link = { kind: "live", city: "sprawling" } as const;
  return (
    <>
      <For each={SHOWN}>
        {(shown) => (
          <GalleryCase label={shown.label} width={shown.width}>
            <Stand
              link={link}
              unread={[]}
              waiting={[]}
              answers={answering}
              commit={PICKED}
              records={records}
              prepare={(preferences) => preferences.setPanel(shown.panel)}
            >
              {() => <ShellSpecimen address={ROOM} tier={shown.tier} height={860} />}
            </Stand>
          </GalleryCase>
        )}
      </For>
      <GalleryCase label="workbench · the city beside the Mayor's room">
        <Stand link={link} unread={[]} waiting={[]} answers={answering} records={records}>
          {() => (
            <div class="flex h-[360px] w-[440px] flex-col">
              <City />
            </div>
          )}
        </Stand>
      </GalleryCase>
      <GalleryCase label="workbench · a building's files beside its room">
        <Stand link={link} unread={[]} waiting={[]} answers={answering} records={records}>
          {() => (
            <div class="flex h-[360px] w-[440px] flex-col">
              <Files here={ROOM} />
            </div>
          )}
        </Stand>
      </GalleryCase>
      <GalleryCase label="listening · who hears a message in this room, as the palette's foot draws it">
        <Stand link={link} unread={[]} waiting={[]} answers={answering} records={records}>
          {() => <Listening room={ROOM} />}
        </Stand>
      </GalleryCase>
      <GalleryCase label="workbench · the chosen session's pane at its panorama width" width={640}>
        <Stand
          link={link}
          unread={[]}
          waiting={[]}
          answers={answering}
          records={records}
          commit={PICKED}
        >
          {() => (
            <div class="flex h-[640px] flex-col">
              <SessionPane lang={lang()} here={ROOM} />
            </div>
          )}
        </Stand>
      </GalleryCase>
      <GalleryCase
        label="workbench · the session's instrument sheet at a phone's width"
        width={390}
      >
        <Stand link={link} unread={[]} waiting={[]} answers={answering} records={records}>
          {() => (
            <SessionSheet
              here={ROOM}
              run={run(6)}
              rounds={{
                run: run(6),
                turns: TURNS,
                opened_at: oid("a"),
                opening: OPENING,
                worktree: "lab-room1",
              }}
            />
          )}
        </Stand>
      </GalleryCase>
    </>
  );
}

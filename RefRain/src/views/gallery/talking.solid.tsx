// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AxError } from "../../wire";
import Coin from "../talk/coin_view";
import DropRefused from "../talk/drop_refused_view";
import type { Refused } from "../talk/dropping";
import Failed from "../talk/failed_view";
import { lookOf as recordLook } from "../talk/record";
import { RecordKey } from "../talk/record_view";
import { GalleryCase } from "./case";

const NO_MODEL: AxError = {
  action: "choose the main model",
  code: "E_MODEL_UNCHOSEN",
  nearby: [],
  recovery: "attach a provider on the settings page and pick a model for this tag",
  retry: "no",
  subject: "no model is chosen for this tag",
};
const FACES = ["send", "stop", "idle"] as const;
const HEARD = [
  { taking: false, hearing: false, refused: false },
  { taking: true, hearing: false, refused: false },
  { taking: false, hearing: true, refused: false },
  { taking: false, hearing: false, refused: true },
] as const;
const NOT_KEPT: readonly Refused[] = [
  { kind: "refused", name: "report.pdf", said: "the file is larger than the city keeps" },
  { kind: "refused", name: "notes.md", said: "" },
];
const ignore = (): void => undefined;

export default function Talking() {
  const lang = storeValue(ui().lang);
  return (
    <>
      <GalleryCase label="coin key · send, stop and the faint face that says why">
        <div class="flex items-center gap-base">
          <For each={FACES}>
            {(face) => <Coin face={face} sending="dispatch" delivery="idle" onStop={ignore} />}
          </For>
        </div>
      </GalleryCase>
      <GalleryCase label="microphone · ready, recording, hearing, refused">
        <div class="flex flex-wrap items-center gap-base">
          <For each={HEARD}>{(heard) => <RecordKey {...recordLook(heard, lang(), ignore)} />}</For>
        </div>
      </GalleryCase>
      <GalleryCase label="composer · files a drop did not keep">
        <DropRefused refused={NOT_KEPT} />
      </GalleryCase>
      <GalleryCase label="conversation · a dispatch refused for want of a model">
        <Failed what={say(lang(), "talk_failed_model")} error={NO_MODEL} onRetry={ignore} />
      </GalleryCase>
    </>
  );
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, onSettled } from "solid-js";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { AxError } from "../../wire";
import Composer from "../talk/composer_view";
import { GalleryCase } from "./case";
import Stand from "./stand.solid";

const LINE = "add a test for price";
const NO_MODEL: AxError = {
  action: "choose the main model",
  code: "E_CONFIG_INVALID",
  nearby: [],
  recovery: "attach a provider on the settings page and pick a model for this tag",
  retry: "no",
  subject: "no model is chosen for this tag",
};

function RefusedComposer(props: { readonly onSend: () => void }) {
  const lang = storeValue(ui().lang);
  let holder: HTMLDivElement | undefined;
  onSettled(() => {
    const box = holder?.querySelector("textarea");
    if (box == null) return;
    box.value = LINE;
    box.dispatchEvent(new Event("input", { bubbles: true }));
    box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
  });
  return (
    <div
      class="px-pane pb-pane"
      ref={(node) => {
        holder = node;
      }}
    >
      <Composer
        placeholder={say(lang(), "talk_placeholder_mayor")}
        sending="dispatch"
        onSend={() => {
          props.onSend();
          return true;
        }}
        onStop={() => false}
      />
    </div>
  );
}

export default function RefusedLine() {
  const [refusing, setRefusing] = createSignal<AxError | null>(null);
  return (
    <GalleryCase label="composer · a refused line back in the box">
      <Stand
        link={{ kind: "live", city: "sprawling" }}
        unread={[]}
        waiting={[]}
        refusing={refusing()}
        children={() => <RefusedComposer onSend={() => setRefusing(NO_MODEL)} />}
      />
    </GalleryCase>
  );
}

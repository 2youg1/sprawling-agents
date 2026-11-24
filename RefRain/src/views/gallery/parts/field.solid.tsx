// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";
import { referenceFor, referenceText } from "../../../core/enrol";
import { say } from "../../../core/lang";
import { ui } from "../../../ui";
import Field from "../../parts/field.solid";
import { GalleryCase } from "../case";
import { storeValue } from "../../../core/solid_store";

export default function FieldCases() {
  const lang = storeValue(ui().lang);
  const [host, setHost] = createSignal("api.example.test");
  const [bad, setBad] = createSignal("api_gateway.internal");
  const [key, setKey] = createSignal(referenceText(referenceFor("example")));
  const [measured, setMeasured] = createSignal("a million");
  const [stepped, setStepped] = createSignal("200000");
  return (
    <>
      <GalleryCase label="field · help, the city's error, the browser's user-invalid, mono with fixed parts">
        <div class="flex flex-col gap-base">
          <Field
            label={say(lang(), "setup_base_url")}
            help={say(lang(), "part_help_base_url")}
            value={host()}
            onInput={setHost}
            prefix="https://"
            suffix="/v1"
          />
          <Field
            label={say(lang(), "setup_base_url")}
            error={say(lang(), "part_error_host")}
            value={bad()}
            onInput={setBad}
          />
          <Field
            label={say(lang(), "part_context_window")}
            help={say(lang(), "part_context_window")}
            value={measured()}
            onInput={setMeasured}
            pattern="[0-9]+"
          />
          <Field label={say(lang(), "setup_key")} value={key()} onInput={setKey} mono />
        </div>
      </GalleryCase>
      <GalleryCase label="field · disabled, a hidden label, a number that steps">
        <div class="flex flex-col gap-base">
          <Field
            label={say(lang(), "setup_key")}
            value={key()}
            onInput={() => undefined}
            mono
            disabled
          />
          <Field
            label={say(lang(), "part_context_window")}
            labelling="hidden"
            kind="number"
            step={1024}
            value={stepped()}
            onInput={setStepped}
          />
        </div>
      </GalleryCase>
    </>
  );
}

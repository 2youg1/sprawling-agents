// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { EndpointsAnswer } from "../../wire";
import { freshTable } from "../setup/model_rows";
import { lookOf } from "../setup/model_table";
import { ModelTableView } from "../setup/model_table.solid";
import Advanced from "../setup/providers/advanced_view";
import { freshDraft } from "../setup/providers/draft";
import type { Draft } from "../setup/providers/draft";
import EndpointList from "../setup/providers/endpoint_list.solid";
import { freshFlight } from "../setup/providers/flight";
import Generation from "../setup/providers/generation.solid";
import { freshGeneration } from "../setup/providers/generation";
import ModelEditor from "../setup/providers/model_editor.solid";
import Previews from "../setup/providers/preview_view";
import { GalleryCase as Case } from "./case";
import { ENDPOINTS, FIGURED, PROBED, TYPED } from "./served";

export default function Attaching() {
  const lang = storeValue(ui().lang);
  const [revision, setRevision] = createSignal(0);
  const changed = () => {
    setRevision((at) => at + 1);
  };

  const LISTED: EndpointsAnswer = {
    chosen: [],
    endpoints: ENDPOINTS.endpoints.map((endpoint, at) =>
      at === 0
        ? {
            ...endpoint,
            base_url: "https://gateway.example.invalid/tenants/house-of-many-models/openai/v1",
          }
        : { ...endpoint, models: [] },
    ),
  };

  const [draft, setDraft] = createSignal<Draft>({
    ...freshDraft(),
    baseUrl: "https://api.example.invalid/v1",
    headers: [{ key: "team", name: "x-team", value: "blue" }],
    overrides: [{ key: "temperature", name: "/temperature", value: "0.2" }],
  });

  const worked = freshTable();
  worked.textOnly = false;
  for (const row of PROBED) worked.ticked[row.id] = true;
  const first = PROBED[0]?.id;
  if (first !== undefined) {
    worked.output[first] = "32768";
    worked.input[first] = "text_image";
    worked.role[first] = "main";
  }

  const typed = freshTable();
  typed.manual = "house/writer, house/reader";
  typed.ticked["house/writer"] = true;

  const answered = freshTable();
  answered.manual = TYPED;

  const workedLook = () => {
    revision();
    return lookOf(worked, PROBED, [], lang());
  };
  const typedLook = () => {
    revision();
    return lookOf(typed, [], [], lang());
  };
  const answeredLook = () => {
    revision();
    return lookOf(answered, PROBED, FIGURED, lang());
  };
  const modelFlight = freshFlight();
  const listedGeneration = LISTED.endpoints[0];
  const generationEndpoint =
    listedGeneration === undefined
      ? undefined
      : {
          ...listedGeneration,
          tuning: {
            ...listedGeneration.tuning,
            overrides: [{ pointer: "/temperature", value: "0.8" }],
          },
        };
  const generationDrafts =
    generationEndpoint === undefined ? {} : freshGeneration(generationEndpoint);
  if (generationEndpoint !== undefined) {
    const first = generationEndpoint.models[0]?.id;
    if (first !== undefined && generationDrafts[first] !== undefined)
      generationDrafts[first].temperature = "0.2";
  }

  return (
    <>
      <Case label="provider · a long address, and a row with no model listed">
        <EndpointList answer={LISTED} />
      </Case>

      <Case label="provider · an attached row's model editor, open with A and B admitted">
        <ModelEditor answer={LISTED} name="example" flight={modelFlight} open />
      </Case>

      <Case label="provider · the attach form's fold, closed">
        <Advanced
          draft={draft()}
          setDraft={(part, value) => {
            setDraft((before) => ({ ...before, [part]: value }));
          }}
          onRenamed={() => undefined}
        />
      </Case>

      <Case label="provider · the two readings of a draft with a header and an override">
        <Previews draft={draft()} reference="secret:providers/example" model="house/writer" />
      </Case>

      {generationEndpoint === undefined ? undefined : (
        <Case label="models · generation defaults, support sources, and an endpoint override">
          <Generation
            models={generationEndpoint.models}
            summary={generationEndpoint}
            drafts={generationDrafts}
            overrideRows={generationEndpoint.tuning.overrides}
            onEdit={() => undefined}
            open
          />
        </Case>
      )}

      <Case label="models · every row ticked, one with its figures and role stated">
        <ModelTableView look={workedLook()} changed={changed} />
      </Case>

      <Case label="models · no list served, two ids typed by hand">
        <ModelTableView look={typedLook()} changed={changed} />
      </Case>

      <Case label="models · what the city will register: a figure and its rung, the provider's default, unknown, and an id named by hand">
        <ModelTableView look={answeredLook()} changed={changed} />
      </Case>
    </>
  );
}

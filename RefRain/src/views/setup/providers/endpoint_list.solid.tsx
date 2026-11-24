// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, Show } from "solid-js";
import { storeValue } from "../../../core/solid_store";
import { ui } from "../../../ui";
import type { EndpointsAnswer } from "../../../wire";
import { HALL } from "../../shared/buildings";
import Accounts from "./accounts.solid";
import { editorState } from "./editor_state";
import { lookOf } from "./endpoint_list";
import Look from "./endpoint_list.look.solid";
import { freshFlight, type FlightState } from "./flight";
import ModelEditor from "./model_editor.solid";
import { endpointRoster } from "./rosters";

export interface EndpointListProps {
  readonly answer: EndpointsAnswer;
  readonly openModels?: boolean | undefined;
}

export default function EndpointList(props: EndpointListProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const config = storeValue(u.conn.asking.ask({ config: { addr: HALL } }));
  const fallback = () => {
    const held = config();
    return held !== undefined && "config" in held ? held.config.tuning.account_retries : undefined;
  };
  const look = createMemo(() => lookOf(props.answer, lang()));
  const flights = new Map<string, FlightState>();
  function flightFor(name: string): FlightState {
    const held = flights.get(name);
    if (held !== undefined) return held;
    const created = editorState(freshFlight());
    flights.set(name, created);
    return created;
  }

  return (
    <Look
      {...look()}
      accounts={(name) => (
        <Show when={props.answer.endpoints.find((each) => each.name === name)}>
          {(endpoint) => (
            <Accounts roster={endpointRoster(endpoint(), fallback())} flight={flightFor(name)} />
          )}
        </Show>
      )}
      models={(name) => (
        <ModelEditor
          answer={props.answer}
          name={name}
          flight={flightFor(name)}
          open={props.openModels}
        />
      )}
    />
  );
}

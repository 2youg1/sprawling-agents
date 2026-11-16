// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One in-flight operation per attached endpoint row. The account and
// model editors hold the same state, so neither can replay a summary
// from before the other editor's command settled.

export type FlightKind = "accounts" | "probe" | "models";

export interface Flight {
  readonly kind: FlightKind;
  readonly ticket: symbol;
}

export interface FlightState {
  current: Flight | null;
}

export function freshFlight(): FlightState {
  return { current: null };
}

export function begin(state: FlightState, kind: FlightKind): Flight | null {
  if (state.current !== null) return null;
  const flight = { kind, ticket: Symbol(kind) };
  state.current = flight;
  return flight;
}

export function end(state: FlightState, flight: Flight): void {
  if (state.current?.ticket === flight.ticket) state.current = null;
}

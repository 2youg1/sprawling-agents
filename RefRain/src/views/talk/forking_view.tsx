// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, onSettled } from "solid-js";
import { fill, say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Call, RoundsAnswer } from "../../wire";
import type { PopoverColumn, PopoverRow } from "../parts/popover";
import Popover from "../parts/popover_view";
import { callWord } from "./calls";
import { planFork, type ForkEntry, type ForkFilter, type ForkPlan } from "./forking";

interface Row {
  readonly id: string;
  readonly entry: ForkEntry;
  readonly label: string;
  readonly secondary?: string;
  readonly tools: boolean;
}

const FILTER_KEY: Readonly<Record<ForkFilter, Key>> = {
  all: "fork_filter_all",
  quiet: "fork_filter_no_tools",
  user: "fork_filter_user",
};
const FILTERS: readonly ForkFilter[] = ["all", "quiet", "user"];

function callRow(call: Call): PopoverRow {
  return { id: `c:${String(call.at)}`, label: callWord(call.tool, call.subject) };
}

export default function Forking(props: {
  readonly rounds: RoundsAnswer;
  readonly onFork: (plan: ForkPlan) => void;
  readonly onClose: () => void;
}) {
  const lang = storeValue(ui().lang);
  const [filter, setFilter] = createSignal<ForkFilter>("all");
  const [cursor, setCursor] = createSignal("");
  const rows = createMemo((): Row[] => {
    const out: Row[] = [];
    const first = props.rounds.turns.at(0);
    const task = props.rounds.opening?.task ?? "";
    if (first !== undefined && task !== "") {
      out.push({
        id: "m:task",
        entry: { kind: "message", turn: first, text: task },
        label: task,
        secondary: say(lang(), "talk_you"),
        tools: first.calls.length > 0,
      });
    }
    for (const turn of props.rounds.turns) {
      const tools = turn.calls.length > 0;
      out.push({
        id: `t:${String(turn.number)}`,
        entry: { kind: "turn", turn },
        label: fill(say(lang(), "fork_from_turn"), { turn: String(turn.number) }),
        secondary: turn.said ?? "",
        tools,
      });
      for (const note of turn.notes) {
        if (!("arrived" in note)) continue;
        out.push({
          id: `m:${String(note.arrived.at)}`,
          entry: { kind: "message", turn, text: note.arrived.said ?? "" },
          label: note.arrived.said ?? "",
          secondary: note.arrived.from ?? "",
          tools,
        });
      }
    }
    return out;
  });
  const shown = createMemo(() => {
    switch (filter()) {
      case "all":
        return rows();
      case "quiet":
        return rows().filter((row) => !row.tools);
      case "user":
        return rows().filter((row) => row.entry.kind === "message");
    }
  });
  const held = createMemo(() => shown().find((row) => row.id === cursor()) ?? shown().at(0));
  const columns = createMemo((): readonly PopoverColumn[] => [
    {
      id: "turns",
      label: "fork_turns",
      rows: shown().map((row) => ({
        id: row.id,
        label: row.label,
        secondary: row.secondary === "" ? undefined : row.secondary,
      })),
    },
    { id: "calls", label: "talk_calls", rows: (held()?.entry.turn.calls ?? []).map(callRow) },
  ]);
  const footer = createMemo(() => {
    const wording = `${say(lang(), FILTER_KEY[filter()])} · ${say(lang(), "fork_keeps")}`;
    const row = held();
    return row !== undefined && planFork(props.rounds.run, row.entry).walkedBack
      ? `${wording} · ${say(lang(), "fork_safe_point")}`
      : wording;
  });
  function choose(column: PopoverColumn, row: PopoverRow): void {
    if (column.id !== "turns") return;
    const selected = shown().find((item) => item.id === row.id);
    if (selected !== undefined) props.onFork(planFork(props.rounds.run, selected.entry));
  }
  function cycle(event: KeyboardEvent): void {
    if (event.isComposing || !event.ctrlKey || event.metaKey || event.key.toLowerCase() !== "o")
      return;
    event.preventDefault();
    setFilter((old) => FILTERS[(FILTERS.indexOf(old) + 1) % FILTERS.length] ?? "all");
  }
  onSettled(() => {
    window.addEventListener("keydown", cycle);
    return () => window.removeEventListener("keydown", cycle);
  });
  return (
    <div class="relative w-full">
      <Popover
        label="fork_pick_title"
        columns={columns()}
        onApply={choose}
        onClose={props.onClose}
        onCursorRow={(row) => {
          if (row !== null && !row.id.startsWith("c:")) setCursor(row.id);
        }}
      />
      <div class="py-tight text-note text-text-faint" role="status">
        {footer()}
      </div>
    </div>
  );
}


// Where each line sits on the axis now drawn: x in [0, 1], or null when
// the axis cannot place it.
function axis() {
  if (state.axis === "order") {
    const n = Math.max(M.events.length - 1, 1);
    return { at: (line) => line.index / n, breaks: [] };
  }
  // A gap longer than GAP_MS is drawn at one width, so a quiet night
  // does not squeeze the minutes around it into a line; it says how
  // long it really was.
  const gaps = M.timed.slice(1).map((line, i) => Math.max(Number(line.moment) - Number(M.timed[i].moment), 0));
  const long = gaps.filter((gap) => gap > GAP_MS).length;
  const busy = gaps.filter((gap) => gap <= GAP_MS).reduce((sum, gap) => sum + gap, 0);
  const shortened = Math.max(busy, 1) / Math.max(8, 2 * long);
  const virtual = new Map();
  const breaks = [];
  let v = 0;
  M.timed.forEach((line, i) => {
    const gap = i === 0 ? 0 : gaps[i - 1];
    if (gap > GAP_MS) { breaks.push({ v: v + shortened / 2, real: gap }); v += shortened; } else v += gap;
    virtual.set(line.seq, v);
  });
  const total = Math.max(v, 1);
  return { at: (line) => (virtual.has(line.seq) ? virtual.get(line.seq) / total : null), breaks: breaks.map((b) => ({ x: b.v / total, real: b.real })), total, virtual };
}

function colour(name) { return getComputedStyle(document.documentElement).getPropertyValue(name).trim(); }

function drawPlot() {
  const canvas = $("plot");
  const lanes = M.laneIds;
  const width = canvas.parentElement.clientWidth;
  const height = lanes.length * LANE + 24;
  const ratio = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.round(width * ratio));
  canvas.height = Math.round(height * ratio);
  canvas.style.height = height + "px";
  const g = canvas.getContext("2d");
  g.setTransform(ratio, 0, 0, ratio, 0, 0);
  g.clearRect(0, 0, width, height);
  const pad = 8;
  const span = Math.max(width - 2 * pad, 1);
  const a = axis();
  const x = (line) => { const at = a.at(line); return at === null ? null : pad + at * span; };
  const laneOf = new Map(lanes.map((id, i) => [id, i]));
  const chapter = chapters().find((c) => c.id === state.chapter);
  const inChapter = (line) => !chapter || (chapter.seqs ? chapter.seqs.has(line.seq) : line.record && line.record.run === chapter.run);
  const c = { rule: colour("--rule"), line: colour("--faint"), ghost: colour("--ghost"), stop: colour("--stop"), warm: colour("--warm"), go: colour("--go"), acid: colour("--acid"), call: colour("--rule-2"), ink: colour("--ink") };
  lanes.forEach((_, i) => { g.fillStyle = c.rule; g.fillRect(0, (i + 1) * LANE - 0.5, width, 1); });
  // calls first, under the marks
  M.calls.forEach((call) => {
    const from = seqIn(call.called, "at") ? M.lines.get(seqIn(call.called, "at")) : null;
    if (!from || !from.record) return;
    const lane = laneOf.get(from.record.run);
    if (lane === undefined) return;
    let x0 = x(from); let x1 = null;
    if (state.axis === "order") {
      const to = seqIn(call.answered, "at") ? M.lines.get(seqIn(call.answered, "at")) : null;
      x1 = to ? x(to) : null;
    } else if (call.took.measured && x0 !== null) {
      x1 = pad + ((a.virtual.get(from.seq) + Number(call.took.measured)) / a.total) * span;
    }
    if (x0 === null || x1 === null) return;
    g.fillStyle = c.call;
    g.fillRect(x0, lane * LANE + 9, Math.max(x1 - x0, 1), 6);
  });
  M.events.forEach((line) => {
    const run = line.record && line.record.run;
    const lane = laneOf.get(run);
    const px = x(line);
    if (lane === undefined || px === null) return;
    const flags = line.flags;
    g.globalAlpha = inChapter(line) ? 1 : 0.25;
    g.fillStyle = flags.includes("refused") || flags.includes("failed") ? c.stop : flags.includes("conflict") ? c.warm : flags.includes("person") ? c.go : flags.includes("commit") ? c.acid : c.line;
    const tall = flags.length > 0;
    g.fillRect(Math.round(px) - 1, lane * LANE + (tall ? 4 : 7), 2, tall ? 16 : 10);
  });
  g.globalAlpha = 1;
  // shortened gaps, each with how long it was
  g.font = "11px " + getComputedStyle(document.body).fontFamily;
  g.textBaseline = "middle";
  a.breaks.forEach((b) => {
    const px = pad + b.x * span;
    g.fillStyle = c.ghost;
    for (let y = 0; y < lanes.length * LANE; y += 4) g.fillRect(Math.round(px), y, 1, 2);
    g.fillText(say("gap", { d: duration(b.real) }), Math.min(px + 4, width - 72), lanes.length * LANE + 12);
  });
  // the playhead
  const current = M.events[state.current];
  if (current) {
    const px = x(current);
    if (px !== null) {
      g.fillStyle = c.acid;
      g.fillRect(Math.round(px) - 0.5, 0, 1, lanes.length * LANE);
      const lane = laneOf.get(current.record && current.record.run);
      if (lane !== undefined) { g.strokeStyle = c.acid; g.lineWidth = 2; g.strokeRect(Math.round(px) - 4, lane * LANE + 3, 8, LANE - 6); }
    }
  }
  if (state.axis === "time" && M.timed.length) {
    g.fillStyle = c.ghost;
    g.fillText(iso(M.timed[0].moment), pad, lanes.length * LANE + 12);
  }
  $("lane-names").querySelectorAll("li").forEach((li, i) => li.classList.toggle("on", current && current.record && lanes[i] === current.record.run));
}

function drawLanes() {
  put($("lane-names"), ...M.laneIds.map((id) => el("li", { title: id }, laneName(id), id === CITY_RUN ? null : [" ", el("small", null, short(id))])));
  $("plot").setAttribute("aria-label", say("plot_label", { lanes: M.laneIds.length, n: M.events.length }));
  const swatch = (name, label) => el("span", null, el("i", { style: "background:var(" + name + ")" }), label);
  put($("legend"), swatch("--faint", say("legend_line")), swatch("--stop", say("legend_flag")), swatch("--warm", say("legend_conflict")), swatch("--go", say("legend_person")), swatch("--acid", say("legend_commit")), swatch("--rule-2", say("legend_call")), el("span", null, say("keys")));
  put($("speeds"), ...SPEEDS.map((speed) => el("button", { type: "button", "aria-pressed": String(state.speed === speed), onclick: () => { state.speed = speed; drawLanes(); } }, speed + "×")));
  const first = M.timed[0]; const last = M.timed[M.timed.length - 1];
  $("timeline-span").textContent = first ? iso(first.moment) + " – " + iso(last.moment) : "";
  drawAxisNote();
}

function drawAxisNote() {
  $("axis-order").setAttribute("aria-pressed", String(state.axis === "order"));
  $("axis-time").setAttribute("aria-pressed", String(state.axis === "time"));
  $("axis-time").disabled = M.timed.length === 0;
  $("axis-note").textContent = state.axis === "order" ? say("axis_order_note")
    : say("axis_time_note", { untimed: M.events.length - M.timed.length });
  if (M.timed.length === 0) $("axis-note").textContent = say("axis_order_note") + " " + say("axis_time_none");
}

function plotClick(event) {
  const canvas = $("plot");
  const box = canvas.getBoundingClientRect();
  const lane = M.laneIds[Math.floor((event.clientY - box.top) / LANE)];
  if (!lane) return;
  const a = axis();
  const pad = 8; const span = Math.max(box.width - 2 * pad, 1);
  let best = null; let bestD = Infinity;
  M.events.forEach((line) => {
    if (!line.record || line.record.run !== lane) return;
    const at = a.at(line);
    if (at === null) return;
    const d = Math.abs(pad + at * span - (event.clientX - box.left));
    if (d < bestD) { bestD = d; best = line; }
  });
  if (best) setCurrent(best.index);
}

// ---------------------------------------------------------------- the playhead

function sequence() { return state.axis === "time" ? M.timed : M.events; }
function setCurrent(index) {
  if (!M.events.length) return;
  state.current = Math.max(0, Math.min(index, M.events.length - 1));
  drawCard(); drawPlot();
  document.querySelectorAll("#log-body tr.on").forEach((tr) => tr.classList.remove("on"));
  const row = $("line-" + M.events[state.current].seq);
  if (row) row.classList.add("on");
}
function step(by) {
  const seq = sequence();
  if (!seq.length) return;
  const here = M.events[state.current];
  const at = state.axis === "time" ? (here.timeIndex ?? -1) : here.index;
  const next = seq[Math.max(0, Math.min(at + by, seq.length - 1))];
  setCurrent(next.index);
}
function jump(toEnd) { const seq = sequence(); if (seq.length) setCurrent(seq[toEnd ? seq.length - 1 : 0].index); }

let timer = null;
function play(on) {
  state.playing = on;
  $("play").setAttribute("aria-pressed", String(on));
  $("play").textContent = say(on ? "pause" : "play");
  $("card").setAttribute("aria-live", on ? "off" : "polite");
  clearTimeout(timer);
  if (on) tick();
}
// One step per beat in order; in time, the wait is the recorded gap
// divided by the speed, never longer than a shortened gap.
function tick() {
  const seq = sequence();
  const here = M.events[state.current];
  const at = state.axis === "time" ? (here.timeIndex ?? -1) : here.index;
  if (at + 1 >= seq.length) { play(false); return; }
  const next = seq[at + 1];
  let wait = 400 / state.speed;
  if (state.axis === "time" && here.moment !== null) wait = Math.min(Math.max(Number(next.moment) - Number(here.moment), 0), GAP_MS) / state.speed;
  timer = setTimeout(() => { setCurrent(next.index); if (state.playing) tick(); }, Math.max(wait, 16));
}

function drawCard() {
  const line = M.events[state.current];
  if (!line) { put($("card"), el("p", { class: "empty" }, say("card_none"))); $("where").textContent = ""; return; }
  const seq = sequence();
  $("where").textContent = state.axis === "time" && line.timeIndex !== undefined
    ? say("where_time", { i: line.timeIndex + 1, n: seq.length, seq: line.seq })
    : say("where", { i: line.index + 1, n: M.events.length, seq: line.seq });
  put($("card"), ...lineCard(line));
}

function lineCard(line) {
  const record = line.record ?? {};
  const call = M.callAt.get(line.seq);
  const rows = [];
  const row = (key, value) => rows.push(el("dt", null, say(key)), el("dd", null, value));
  row("card_time", line.moment !== null
    ? el("span", null, iso(line.moment), el("div", { class: "note" }, say("card_time_note")))
    : el("span", { class: "f" }, say("card_no_time"), el("div", { class: "note" }, say(MOMENT_KINDS.has(record.kind) ? "card_unmeasured_note" : "card_untimed_note"))));
  if (record.run) row("card_run", runLabel(record.run));
  if (record.who) row("card_who", record.who);
  if (record.addr) row("card_addr", record.addr);
  if (call) {
    const callee = call.callee.tool ? say("callee_tool", { name: call.callee.tool.name ?? say("name_unseen") }) : say("callee_model", { name: call.callee.model.name ?? say("name_unseen") });
    row("card_call", el("span", null, callee, " · ", end(call.called), " → ", end(call.answered), " · ",
      call.took.measured ? duration(call.took.measured) : el("span", { class: "f", title: say("took_note") }, say("took_unknown"))));
  }
  const said = [...$("narration").querySelectorAll("[data-seq]")].filter((node) => node.getAttribute("data-seq") === line.seq)
    .map((node) => node.closest("p, li, blockquote, div") ?? node);
  return [
    el("div", { class: "head" }, el("b", null, record.kind ?? "?"), cite(line.seq), el("span", { class: "f" }, say(line.place === "event" ? "card_place_event" : "card_place_context")), line.flags.map(tag)),
    summaryOf(line) ? el("p", { class: "q", style: "margin-top:8px" }, summaryOf(line)) : null,
    el("dl", null, rows),
    said.length ? el("div", { class: "said" }, el("p", { class: "lbl" }, say("card_said")), said.map((node) => el("p", null, node.textContent))) : null,
    el("details", { style: "margin-top:16px" }, el("summary", null, say("card_line")), el("pre", { class: "line" }, line.line)),
  ];
}

// Shows the line at `seq` wherever it is: the playhead if it is in the
// selection, the log row in any case.
function show(seq) {
  const line = M.lines.get(seq);
  if (!line) return;
  if (line.place === "event") setCurrent(line.index);
  if (line.place === "context" && !state.context) state.context = true;
  let rows = filtered();
  if (!rows.includes(line)) { state.filter = "all"; state.run = ""; state.chapter = ""; state.text = ""; drawFilters(); drawChapters(); drawPlot(); rows = filtered(); }
  const at = rows.indexOf(line);
  if (at >= state.shown) state.shown = Math.ceil((at + 1) / PAGE) * PAGE;
  drawFilters(); drawLog();
  const row = $("line-" + seq);
  if (row) { row.scrollIntoView({ block: "center", behavior: REDUCED.matches ? "auto" : "smooth" }); row.querySelector("button").focus({ preventScroll: true }); }
}

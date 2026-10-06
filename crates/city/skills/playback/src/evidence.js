// ---------------------------------------------------------------- the page

function drawHeader() {
  const { bundle, events } = M;
  const src = bundle.source;
  const reader = src.reader.person
    ? say(src.reader.person === "included" ? "reader_person_all" : "reader_person")
    : say("reader_resident", { addr: src.reader.resident });
  $("title").textContent = say("title_city", { city: String(src.city).slice(0, 12) });
  $("subtitle").textContent = say("subtitle", {
    from: events.length ? events[0].seq : "—", through: events.length ? events[events.length - 1].seq : "—",
    cutoff: src.cutoff.seq, reader,
    span: M.timed.length ? say("span", { first: iso(M.timed[0].moment), last: iso(M.timed[M.timed.length - 1].moment) }) : say("span_none"),
  });
  document.title = say("playback") + " · " + say("title_city", { city: String(src.city).slice(0, 12) });
  const measured = M.calls.filter((call) => call.took && call.took.measured).length;
  const commits = bundle.checkpoints.filter((c) => c.holds.committed).length;
  const fact = (label, value, small) => el("div", null, el("dt", { class: "lbl" }, label), el("dd", null, value, small ? el("small", null, " " + small) : null));
  put($("facts"),
    fact(say("fact_events"), thousands(events.length), bundle.context.length ? say("fact_context", { n: bundle.context.length }) : null),
    fact(say("fact_runs"), thousands(bundle.runs.length)),
    fact(say("fact_calls"), thousands(M.calls.length), say("fact_measured", { n: measured })),
    fact(say("fact_commits"), thousands(commits)),
    fact(say("fact_messages"), thousands(bundle.messages.length)),
    fact(say("fact_cost"), usd(bundle.costs.billed_usd_micros), big(bundle.costs.unpriced_calls) > 0n ? say("fact_unpriced", { n: bundle.costs.unpriced_calls }) : null),
    fact(say("fact_withheld"), thousands(bundle.withheld.events)),
    fact(say("fact_timed"), thousands(M.timed.length), say("fact_timed_of", { m: events.length })),
  );
}

function drawNarration() {
  const section = $("narration");
  const said = [...section.children].filter((child) => child.tagName !== "H2");
  section.hidden = said.length === 0;
  section.querySelectorAll("[data-seq]").forEach((node) => {
    node.setAttribute("role", "button");
    node.tabIndex = 0;
    const go = () => show(node.getAttribute("data-seq"));
    node.addEventListener("click", go);
    node.addEventListener("keydown", (event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); go(); } });
  });
}

function drawRuns() {
  const { bundle } = M;
  $("runs-count").textContent = String(bundle.runs.length);
  const counts = { refused: 0, failed: 0, conflict: 0, person: 0 };
  M.events.forEach((line) => line.flags.forEach((flag) => { if (flag in counts) counts[flag] += 1; }));
  const items = [];
  bundle.moments.filter((m) => m.family === "approval" && m.closed === "pending").forEach((m) =>
    items.push(el("li", null, tag("waiting"), say("att_approval"), " ", end(m.opened))));
  bundle.moments.filter((m) => m.family === "pr" && m.closed === "pending").forEach((m) =>
    items.push(el("li", null, tag("waiting"), say("att_pr"), " ", el("span", { class: "q" }, m.key), " ", end(m.opened))));
  bundle.runs.filter((run) => big(run.unanswered) > 0n).forEach((run) =>
    items.push(el("li", null, tag("waiting"), say("att_unanswered", { n: run.unanswered, run: (run.addr ?? "") + " " + short(run.run) }))));
  Object.entries(counts).filter(([, n]) => n > 0).forEach(([flag, n]) =>
    items.push(el("li", null, tag(flag), el("button", { type: "button", class: "link", onclick: () => { setFilter(flag); $("log").scrollIntoView(); } }, say("att_flag", { n, tag: say("tag_" + flag) })))));
  put($("attention"), el("li", { class: "lbl" }, say("attention")), ...(items.length ? items : [el("li", { class: "f" }, say("nothing_waits"))]));
  const cols = [{ key: "col_room", nw: true }, { key: "col_run" }, { key: "col_task" }, { key: "col_state", nw: true }, { key: "col_outcome" }, { key: "col_policy" }, { key: "col_lines", num: true }, { key: "col_cost", num: true }];
  const rows = bundle.runs.map((run) => {
    const held = M.ofRun.get(run.run) ?? { count: 0, started: null, frozen: null };
    const started = held.started;
    const task = started ? started.record.data.task : null;
    const frozen = held.frozen;
    const outcome = run.state === "frozen"
      ? (frozen ? (frozen.record.data.cause ? say("outcome_died") : say("outcome_frozen", { why: frozen.record.data.completion ?? "" })) : say("state_frozen"))
      : run.state === "active" ? say("outcome_running") : say("state_unknown");
    const relations = [
      relatedText(run.parent, "child_of"),
      run.forked_at ? say("forked", { run: run.parent && run.parent.run ? short(run.parent.run) : "?", seq: run.forked_at }) : null,
      relatedText(run.predecessor, "after"),
    ].filter(Boolean);
    return el("tr", null,
      cell(cols[0], el("span", null, el("span", { class: "dot " + (run.state ?? "") }), " ", run.addr ?? say("no_addr"))),
      cell(cols[1], el("button", { type: "button", class: "cite", title: run.run, onclick: () => { setChapter("run:" + run.run); } }, short(run.run))),
      cell(cols[2], el("span", null, task ? el("span", { title: String(task) }, clip(String(task), 140), " ", cite(started.seq)) : el("span", { class: "f" }, say("task_unseen")), relations.length ? el("div", { class: "note" }, relations.join(" · ")) : null)),
      cell(cols[3], el("span", null, say(run.state ? "state_" + run.state : "state_unknown"), big(run.unanswered) > 0n ? el("div", { class: "note" }, say("unanswered", { n: run.unanswered })) : null)),
      cell(cols[4], el("span", null, outcome, " ", frozen ? cite(frozen.seq) : null)),
      cell(cols[5], el("span", { class: "q small" }, policyText(run.policy))),
      cell(cols[6], el("span", null, cite(run.first_seq), " – ", cite(run.last_seq), el("div", { class: "note" }, thousands(held.count)))),
      cell(cols[7], M.cost.has(run.run) ? usd(M.cost.get(run.run)) : "—"),
    );
  });
  put($("runs-body"), rows.length ? table(cols, rows, { stack: true }) : el("p", { class: "empty" }, "—"));
}

function chapters() {
  const list = [];
  M.bundle.runs.forEach((run) => list.push({ id: "run:" + run.run, label: run.addr ?? say("no_addr"), sub: say("family_run") + " " + short(run.run), seqs: null, run: run.run }));
  M.bundle.moments.filter((m) => m.family !== "run").forEach((m) => list.push({ id: m.family + ":" + m.key, label: m.key, sub: say("family_" + m.family), seqs: new Set(m.seqs) }));
  return list;
}

function drawChapters() {
  const list = chapters();
  put($("chapter-list"), ...list.map((chapter) => el("li", null,
    el("button", { type: "button", "aria-pressed": String(state.chapter === chapter.id), onclick: () => setChapter(state.chapter === chapter.id ? "" : chapter.id) },
      el("span", { class: "dot" }), el("span", { class: "mono-cut" }, chapter.label), el("small", null, chapter.sub)))));
}

function drawMoments() {
  const moments = M.bundle.moments;
  $("moments-count").textContent = String(moments.length);
  if (!moments.length) { put($("moments-body"), el("p", { class: "empty" }, say("moment_none"))); return; }
  const cols = [{ key: "col_family" }, { key: "col_key" }, { key: "col_opened" }, { key: "col_closed" }, { key: "col_members", num: true }];
  const rows = moments.map((m) => {
    const id = m.family + ":" + m.key;
    const closing = seqIn(m.closed, "at") ? M.lines.get(seqIn(m.closed, "at")) : null;
    const outcome = closing && closing.record ? summaryOf(closing) : m.family === "pr" && m.closed === "pending" ? say("pr_open") : "";
    return el("tr", { class: state.chapter === id || state.chapter === "run:" + m.key ? "on" : null },
      cell(cols[0], say("family_" + m.family)),
      cell(cols[1], el("span", null, el("button", { type: "button", class: "link", onclick: () => setChapter(m.family === "run" ? "run:" + m.key : id) }, m.family === "run" ? (runLabel(m.key)) : m.key), outcome ? el("div", { class: "note" }, outcome) : null)),
      cell(cols[2], end(m.opened)),
      cell(cols[3], end(m.closed)),
      cell(cols[4], thousands(m.seqs.length)),
    );
  });
  put($("moments-body"), table(cols, rows, { stack: true }));
}

function drawCommits() {
  const points = M.bundle.checkpoints;
  $("commits-count").textContent = String(points.length);
  if (!points.length) { put($("commits-body"), el("p", { class: "empty" }, say("commits_none"))); return; }
  put($("commits-body"), ...points.map(commitView));
}

function commitView(point) {
  const holds = point.holds;
  const kind = holds.pinned ? "commit_pinned" : holds.merged ? "commit_merged" : "commit_committed";
  const oid = holds.committed ? holds.committed.oid : holds.merged ? holds.merged.oid : null;
  const fact = (key, value) => el("span", { class: "fact" }, el("span", { class: "lbl" }, say(key)), " ", value);
  const facts = [];
  if (holds.pinned) facts.push(el("span", { class: "fact" }, say("commit_job", { job: holds.pinned.job })));
  if (holds.merged) {
    const line = M.lines.get(point.seq);
    if (line && line.record) facts.push(fact("commit_outcome", summaryOf(line)));
  }
  const c = holds.committed;
  if (c) {
    facts.push(fact("commit_base", c.base === "none" ? say("base_none") : c.base.previous !== undefined ? say("base_previous", { oid: shortOid(c.base.previous) }) : say("base_parent", { oid: shortOid(c.base.parent) })));
    let trace;
    if (c.trace === "untraced") trace = say("trace_untraced");
    else if (c.trace.unread) trace = say("trace_unread", { code: c.trace.unread });
    else {
      const named = c.trace.traced.calls.map((call) => seqIn(call, "at") ? cite(seqIn(call, "at"))
        : seqIn(call, "elsewhere") ? el("span", { class: "end pending" }, say("trace_elsewhere", { seq: seqIn(call, "elsewhere") }))
        : el("span", { class: "end withheld" }, say("trace_withheld")));
      trace = named.length ? named : say("trace_none");
    }
    facts.push(fact("commit_trace", trace));
    if (c.scope.length) facts.push(fact("commit_scope", c.scope.join(" ")));
    if (c.trace.traced && c.trace.traced.nearby.length) {
      facts.push(fact("commit_nearby", el("span", { title: say("nearby_note") },
        c.trace.traced.nearby.map((near) => say("nearby_row", { who: near.actor ?? (near.run && near.run.run ? short(near.run.run) : relatedText(near.run, "after")), n: near.calls })).join(" · "),
        el("span", { class: "f" }, " (" + say("nearby_note") + ")"))));
    }
  }
  return el("article", { class: "checkpoint" },
    el("span", { class: "tag " + (holds.pinned ? "waiting" : holds.merged ? "person" : "commit") }, say(kind)),
    el("div", null,
      el("div", { class: "head" },
        oid ? el("b", { class: "oid", title: oid }, shortOid(oid)) : null,
        el("span", { class: "f" }, runLabel(point.run)),
        cite(point.seq),
        c ? el("span", { class: "f" }, say("commit_files") + " " + thousands(c.files.length)) : null),
      el("p", { class: "facts-line" }, facts),
      c ? c.diff.map(fileView) : null));
}

function fileView(file) {
  const change = file.change;
  const kind = typeof change === "string" ? change : Object.keys(change)[0];
  const body = typeof change === "string" ? null : change[kind];
  let label;
  if (kind === "patch") {
    const add = body.lines.filter((l) => l.text.startsWith("+") && !l.text.startsWith("+++")).length;
    const del = body.lines.filter((l) => l.text.startsWith("-") && !l.text.startsWith("---")).length;
    label = say("change_patch", { add, del });
  } else if (kind === "truncated") label = say("change_truncated", { shown: body.lines.length, cut: body.cut });
  else label = say("change_" + kind);
  const summary = el("summary", null, el("span", { class: "path" }, file.path), el("span", { class: kind === "withheld" ? "end withheld" : kind === "missing" ? "end missing" : "f" }, label));
  if (!body) return el("details", { class: "file" }, summary);
  const rows = [];
  const held = new Map(body.credential.map((h) => [h.number, h.reason]));
  const numbers = [...body.lines.map((l) => l.number), ...held.keys()].sort(bySeq);
  const texts = new Map(body.lines.map((l) => [l.number, l.text]));
  numbers.forEach((number) => {
    if (held.has(number)) { rows.push(el("div", { class: "held" }, el("span", null, number), el("span", null, say("held_line", { n: number, why: held.get(number) })))); return; }
    const text = texts.get(number);
    const cls = text.startsWith("@@") ? "hunk" : text.startsWith("+") && !text.startsWith("+++") ? "add" : text.startsWith("-") && !text.startsWith("---") ? "del" : null;
    rows.push(el("div", { class: cls }, el("span", null, number), el("span", null, text)));
  });
  if (kind === "truncated") rows.push(el("div", { class: "cut" }, el("span", null, "…"), el("span", null, say("cut_lines", { n: body.cut }))));
  return el("details", { class: "file" }, summary, el("div", { class: "diff" }, rows));
}

function drawMessages() {
  const messages = M.bundle.messages;
  $("messages-count").textContent = String(messages.length);
  if (!messages.length) { put($("messages-body"), el("p", { class: "empty" }, say("messages_none"))); return; }
  const cols = [{ key: "col_id" }, { key: "col_from" }, { key: "col_to" }, { key: "col_sent" }, { key: "col_consumed" }];
  put($("messages-body"), table(cols, messages.map((m) => el("tr", null,
    cell(cols[0], m.id), cell(cols[1], m.from ?? "—"), cell(cols[2], m.room ?? "—"),
    cell(cols[3], end(m.sent)), cell(cols[4], end(m.consumed)))), { stack: true }));
}

let callsShown = PAGE;
function drawCalls() {
  const calls = M.calls.filter((call) => !state.run || call.run === state.run);
  $("calls-count").textContent = String(M.calls.length);
  if (!M.calls.length) { put($("calls-body"), el("p", { class: "empty" }, say("calls_none"))); return; }
  const cols = [{ key: "col_run" }, { key: "col_callee" }, { key: "col_called" }, { key: "col_answered" }, { key: "col_took", num: true }];
  const rows = calls.slice(0, callsShown).map((call) => {
    const callee = call.callee.tool ? say("callee_tool", { name: call.callee.tool.name ?? say("name_unseen") }) : say("callee_model", { name: call.callee.model.name ?? say("name_unseen") });
    const took = call.took.measured ? say("took_measured", { d: duration(call.took.measured) }) : el("span", { class: "f", title: say("took_note") }, say("took_unknown"));
    return el("tr", null, cell(cols[0], runLabel(call.run)), cell(cols[1], callee), cell(cols[2], end(call.called)), cell(cols[3], end(call.answered)), cell(cols[4], took));
  });
  const more = calls.length > callsShown
    ? el("button", { type: "button", class: "more", onclick: () => { callsShown += PAGE; drawCalls(); } }, say("show_more", { n: Math.min(PAGE, calls.length - callsShown) }))
    : null;
  const pendingModel = calls.some((call) => call.callee.model && call.answered === "pending");
  put($("calls-body"), table(cols, rows, { stack: true }), more, el("p", { class: "note" }, say("took_note")), pendingModel ? el("p", { class: "note" }, say("model_pending_note")) : null);
}

// ---------------------------------------------------------------- the log

function filtered() {
  const chapter = chapters().find((c) => c.id === state.chapter);
  const text = state.text.toLowerCase();
  const pool = state.context ? [...M.lines.values()].sort((a, b) => bySeq(a.seq, b.seq)) : M.events;
  return pool.filter((line) => {
    const run = line.record && line.record.run;
    if (state.run && run !== state.run) return false;
    if (chapter && chapter.seqs && !chapter.seqs.has(line.seq)) return false;
    if (chapter && chapter.run && run !== chapter.run) return false;
    if (state.filter !== "all" && !line.flags.includes(state.filter)) return false;
    if (text) {
      const hay = [line.seq, line.record && line.record.kind, line.record && line.record.who, line.record && line.record.addr, summaryOf(line)].join(" ").toLowerCase();
      if (!hay.includes(text)) return false;
    }
    return true;
  });
}

function drawFilters() {
  const kinds = ["all", "refused", "failed", "conflict", "person", "waiting", "commit"];
  const runs = el("select", { "aria-label": say("filter_run"), onchange: (event) => { state.run = event.target.value; state.shown = PAGE; callsShown = PAGE; drawLog(); drawCalls(); } },
    el("option", { value: "" }, say("all_runs")),
    M.laneIds.map((id) => el("option", { value: id, selected: state.run === id }, runLabel(id))));
  const search = el("input", { type: "search", placeholder: say("filter_text"), "aria-label": say("filter_text"), value: state.text, oninput: (event) => { state.text = event.target.value; state.shown = PAGE; drawLog(); } });
  const ctx = el("label", null, el("input", { type: "checkbox", checked: state.context, onchange: (event) => { state.context = event.target.checked; state.shown = PAGE; drawLog(); } }), say("filter_context"));
  put($("filters"),
    el("div", { class: "seg", role: "group", "aria-label": say("filter_all") }, kinds.map((kind) =>
      el("button", { type: "button", "aria-pressed": String(state.filter === kind), onclick: () => setFilter(kind) }, kind === "all" ? say("filter_all") : say("tag_" + kind)))),
    runs, search, ctx);
}

function drawLog() {
  const rows = filtered();
  $("log-count").textContent = say("shown", { shown: thousands(Math.min(rows.length, state.shown)), all: thousands(rows.length) });
  const cols = [{ key: "col_seq", nw: true }, { key: "col_time", nw: true }, { key: "col_run", nw: true }, { key: "col_kind" }, { key: "col_who" }, { key: "col_what" }];
  const current = M.events[state.current];
  const body = rows.slice(0, state.shown).map((line) => {
    const record = line.record ?? {};
    return el("tr", { id: "line-" + line.seq, class: [line === current ? "on" : "", line.place === "context" ? "ctx" : ""].join(" ").trim() || null },
      cell(cols[0], cite(line.seq)),
      cell(cols[1], line.moment !== null ? iso(line.moment) : el("span", { class: "f" }, "—")),
      cell(cols[2], record.run ? runLabel(record.run) : "—"),
      cell(cols[3], el("span", null, record.kind ?? "?", " ", line.flags.map(tag))),
      cell(cols[4], record.addr && record.addr !== record.who ? record.who + " · " + record.addr : record.who),
      cell(cols[5], summaryOf(line)));
  });
  const unknown = M.bundle.unknown.length ? el("p", { class: "note" }, say("unknown_seqs", { n: M.bundle.unknown.length, seqs: M.bundle.unknown.join(", ") })) : null;
  const more = rows.length > state.shown
    ? el("button", { type: "button", class: "more", onclick: () => { state.shown += PAGE; drawLog(); } }, say("show_more", { n: Math.min(PAGE, rows.length - state.shown) }))
    : null;
  put($("log-body"), table(cols, body, { stack: true }), more, unknown);
}

function setFilter(kind) { state.filter = kind; state.shown = PAGE; drawFilters(); drawLog(); }
function setChapter(id) {
  state.chapter = id; state.shown = PAGE;
  drawChapters(); drawMoments(); drawLog(); drawPlot();
  const chapter = chapters().find((c) => c.id === id);
  if (chapter) {
    const first = M.events.find((line) => chapter.seqs ? chapter.seqs.has(line.seq) : line.record && line.record.run === chapter.run);
    if (first) setCurrent(first.index);
  }
}

// ---------------------------------------------------------------- cost, withheld, source

function drawCost() {
  const costs = M.bundle.costs;
  const cols = [{ key: "col_run" }, { key: "col_cost", num: true }];
  put($("cost-body"),
    el("p", { style: "margin-bottom:16px" }, el("span", { class: "lbl" }, say("cost_total")), " ", el("b", null, usd(costs.billed_usd_micros))),
    costs.by_run.length ? table(cols, costs.by_run.map((row) => el("tr", null, cell(cols[0], runLabel(row.run)), cell(cols[1], usd(row.usd_micros))))) : null,
    big(costs.unpriced_calls) > 0n || big(costs.unpriced_tokens) > 0n ? el("p", { class: "note" }, say("cost_unpriced", { calls: thousands(costs.unpriced_calls), tokens: thousands(costs.unpriced_tokens) })) : null,
    el("p", { class: "note" }, say("cost_cover")));
}

function drawWithheld() {
  const w = M.bundle.withheld;
  const nothing = big(w.events) === 0n && big(w.credential) === 0n && !w.buildings.length && !w.kinds.length;
  if (nothing) { put($("withheld-body"), el("p", { class: "empty" }, say("withheld_none"))); return; }
  const cols = [{ key: "col_room" }, { key: "col_what" }];
  const kinds = [{ key: "col_kind" }, { key: "col_lines", num: true }];
  put($("withheld-body"),
    el("p", null, say("withheld_events", { n: thousands(w.events) })),
    big(w.credential) > 0n ? el("p", null, say("withheld_credential", { n: thousands(w.credential) })) : null,
    w.buildings.length ? [el("p", { class: "lbl", style: "margin-top:16px" }, say("withheld_buildings")), table(cols, w.buildings.map((b) => el("tr", null, cell(cols[0], b.building), cell(cols[1], say("reason_" + b.reason)))))] : null,
    w.kinds.length ? [el("p", { class: "lbl", style: "margin-top:16px" }, say("withheld_kinds")), table(kinds, w.kinds.map((k) => el("tr", null, cell(kinds[0], k.kind), cell(kinds[1], thousands(k.count)))))] : null,
    el("p", { class: "note" }, say("withheld_note")));
}

function drawSource() {
  const src = M.bundle.source;
  const sel = src.selection;
  const parts = [
    sel.from && say("sel_from", { v: sel.from }), sel.through && say("sel_through", { v: sel.through }),
    sel.run && say("sel_run", { v: sel.run }), sel.building && say("sel_building", { v: sel.building }),
    sel.since && say("sel_since", { v: iso(sel.since) }), sel.until && say("sel_until", { v: iso(sel.until) }),
  ].filter(Boolean);
  const reader = src.reader.person ? say(src.reader.person === "included" ? "reader_person_all" : "reader_person") : say("reader_resident", { addr: src.reader.resident });
  const row = (key, value) => [el("dt", null, say(key)), el("dd", null, value)];
  put($("source-body"), el("div", { class: "card", style: "margin-top:0" }, el("dl", null,
    row("src_schema", M.bundle.schema), row("src_city", src.city),
    row("src_selection", parts.length ? parts.join(" · ") : say("sel_everything")),
    row("src_cutoff", el("span", null, "seq " + src.cutoff.seq + " · ", el("span", { class: "f" }, src.cutoff.chain_hash))),
    row("src_rules", String(src.rules)), row("src_reader", reader),
    row("src_check", el("code", null, say("src_check_line"))))), el("p", { class: "note" }, say("src_note")));
}

// ---------------------------------------------------------------- the lanes

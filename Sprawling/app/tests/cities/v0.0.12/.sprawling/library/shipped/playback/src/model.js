const SCHEMA = "sprawling.playback/3";
const CITY_RUN = "00000000-0000-0000-0000-000000000000";
// The kinds of line that record the moment their own event happened.
const MOMENT_KINDS = new Set(["model_called", "model_returned", "tool_called", "tool_result"]);
// How the lanes look and move.
const LANE = 24;
const GAP_MS = 60000;
const PAGE = 200;
const SPEEDS = [1, 10, 100];
const REDUCED = matchMedia("(prefers-reduced-motion: reduce)");

// ---------------------------------------------------------------- reading

// A ledger line's own JSON, with every integer too large for a Number
// kept as the digits it was written with.
function parseLine(text) {
  return JSON.parse(text, (_, value, context) =>
    typeof value === "number" && !Number.isSafeInteger(value) && context && typeof context.source === "string"
      ? context.source : value);
}
const big = (decimal) => BigInt(decimal);
// The seq an end or a cited call names, when it names one: ends are
// objects ({"at"}, {"outside"}, {"elsewhere"}) or one of the plain words.
const seqIn = (end, key) => (end !== null && typeof end === "object" && typeof end[key] === "string" ? end[key] : null);
const bySeq = (a, b) => (big(a) < big(b) ? -1 : big(a) > big(b) ? 1 : 0);
const el = (tag, props, ...children) => {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props ?? {})) {
    if (value === undefined || value === null || value === false) continue;
    if (key === "class") node.className = value;
    else if (key === "text") node.textContent = value;
    else if (key.startsWith("on")) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value === true ? "" : String(value));
  }
  for (const child of children.flat()) if (child !== null && child !== undefined && child !== false) node.append(child);
  return node;
};
const $ = (id) => document.getElementById(id);
// Replaces a node's children, skipping the absent ones a view leaves out.
const put = (node, ...children) => node.replaceChildren(...children.flat(Infinity).filter((child) => child !== null && child !== undefined && child !== false));

function readBundle() {
  const text = $("playback-bundle").textContent.trim();
  if (text === "") return { problem: say("empty_bundle") };
  let data;
  try { data = JSON.parse(text); } catch (err) { return { problem: say("bad_bundle", { why: err.message }) }; }
  if (data === null || typeof data !== "object") return { problem: say("bad_bundle", { why: typeof data }) };
  if (data.schema !== SCHEMA) return { problem: say("bad_schema", { want: SCHEMA, got: String(data.schema) }) };
  return { bundle: data };
}

// Everything the page draws, derived once from the bundle.
function model(bundle) {
  const lines = new Map();
  const add = (entry, place) => {
    let record = null;
    try { record = parseLine(entry.line); } catch (_) { record = null; }
    lines.set(entry.seq, { seq: entry.seq, moment: entry.moment, line: entry.line, place, record });
  };
  bundle.events.forEach((entry) => add(entry, "event"));
  bundle.context.forEach((entry) => add(entry, "context"));
  const events = bundle.events.map((entry) => lines.get(entry.seq));
  events.forEach((line, index) => { line.index = index; });
  const runs = new Map(bundle.runs.map((run) => [run.run, run]));
  const calls = bundle.calls;
  const callAt = new Map();
  calls.forEach((call) => {
    [call.called, call.answered].forEach((end) => { if (seqIn(end, "at")) callAt.set(seqIn(end, "at"), call); });
  });
  const cost = new Map(bundle.costs.by_run.map((row) => [row.run, row.usd_micros]));
  const laneIds = [...runs.keys()];
  if (events.some((line) => line.record && line.record.run === CITY_RUN) && !runs.has(CITY_RUN)) laneIds.unshift(CITY_RUN);
  events.forEach((line) => { const run = line.record && line.record.run; if (run && !laneIds.includes(run)) laneIds.push(run); });
  laneIds.sort((a, b) => {
    if (a === CITY_RUN) return -1;
    if (b === CITY_RUN) return 1;
    const ra = runs.get(a); const rb = runs.get(b);
    return ra && rb ? bySeq(ra.first_seq, rb.first_seq) : 0;
  });
  // What each run holds, and what each line answers to, read once.
  const ofRun = new Map();
  lines.forEach((line) => {
    const run = line.record && line.record.run;
    if (!run) return;
    const held = ofRun.get(run) ?? { count: 0, started: null, frozen: null };
    if (line.place === "event") held.count += 1;
    if (line.record.kind === "run_started") held.started = line;
    if (line.record.kind === "run_frozen" && line.place === "event") held.frozen = line;
    ofRun.set(run, held);
  });
  const open = new Set([
    ...bundle.moments.filter((m) => m.closed === "pending").map((m) => seqIn(m.opened, "at")),
    ...bundle.messages.filter((m) => m.consumed === "pending").map((m) => seqIn(m.sent, "at")),
    ...calls.filter((c) => c.answered === "pending").map((c) => seqIn(c.called, "at")),
  ]);
  lines.forEach((line) => { line.flags = flagsOf(line); if (open.has(line.seq)) line.flags.push("waiting"); });
  const timed = events.filter((line) => line.moment !== null).sort((a, b) => bySeq(a.moment, b.moment) || bySeq(a.seq, b.seq));
  timed.forEach((line, index) => { line.timeIndex = index; });
  return { bundle, lines, events, runs, calls, callAt, cost, laneIds, timed, ofRun };
}

// ---------------------------------------------------------------- facts

const short = (id) => (id === CITY_RUN ? say("city_lane") : "…" + String(id).slice(-6));
const shortOid = (oid) => String(oid).slice(0, 12);
function iso(ms) {
  if (ms === null || ms === undefined) return null;
  const n = Number(ms);
  return Number.isSafeInteger(n) ? new Date(n).toISOString() : String(ms);
}
function duration(ms) {
  const n = Number(ms);
  if (!Number.isSafeInteger(n)) return say("ms", { n: ms });
  if (n < 1000) return say("ms", { n });
  if (n < 60000) return say("s", { n: (n / 1000).toFixed(n < 10000 ? 3 : 1) });
  if (n < 3600000) return say("min", { m: Math.floor(n / 60000), s: String(Math.floor(n / 1000) % 60).padStart(2, "0") });
  if (n < 86400000) return say("h", { h: Math.floor(n / 3600000), m: String(Math.floor(n / 60000) % 60).padStart(2, "0") });
  return say("d", { d: Math.floor(n / 86400000), h: Math.floor(n / 3600000) % 24 });
}
function usd(micros) {
  const m = big(micros);
  const whole = m / 1000000n;
  const part = String(m % 1000000n).padStart(6, "0").replace(/0{1,4}$/, "");
  return "$" + whole.toString() + "." + part;
}
const thousands = (decimal) => String(decimal).replace(/\B(?=(\d{3})+(?!\d))/g, ",");

// Which of the reader's filters a line answers to: the one table of them.
function flagsOf(line) {
  const record = line.record;
  if (!record) return [];
  const data = record.data ?? {};
  const flags = [];
  switch (record.kind) {
    case "gate_denied": case "budget_limit": case "secret_egress_blocked": case "backpressure_shed": case "pr_rejected":
      flags.push("refused"); break;
    case "provider_degraded": if (data.error || data.code) flags.push("refused"); break;
    case "approval_resolved": if (data.verdict === "deny") flags.push("refused"); break;
    case "tool_result": if (data.error) flags.push("failed"); break;
    case "watchdog_fired": case "roadmap_blocked": case "endpoint_lost": flags.push("failed"); break;
    case "run_frozen": if (data.cause) flags.push("failed"); break;
    case "goal_conflict": case "arbitration_verdict": flags.push("conflict"); break;
    case "checkpoint_committed": if (data.oid) flags.push("commit"); break;
    case "pr_merged": flags.push("commit"); break;
    default: break;
  }
  if (record.who === "person" || record.kind === "steer_received" || record.kind === "cancel_received") flags.push("person");
  return flags;
}

// One line of what a record says, for the log and the card.
function summaryOf(line) {
  const record = line.record;
  if (!record) return "";
  const data = record.data ?? {};
  const code = (err) => (err && typeof err === "object" ? [err.code, err.subject].filter(Boolean).join(" ") : "");
  let text;
  switch (record.kind) {
    case "run_started": text = data.task; break;
    case "tool_called": text = [data.name, data.subject].filter(Boolean).join(" "); break;
    case "tool_result": text = data.error ? [data.name, code(data.error)].join(" ") : data.name; break;
    case "model_called": text = data.model; break;
    case "model_returned": text = [data.stop, data.usage ? Object.entries(data.usage).map(([k, v]) => k + " " + v).join(" · ") : ""].filter(Boolean).join(" · "); break;
    case "signal_enqueued": text = [data.from, "→", data.room].join(" "); break;
    case "signal_consumed": text = data.by; break;
    case "checkpoint_committed": text = data.oid ? shortOid(data.oid) + " " + (data.files ?? []).join(" ") : data.job; break;
    case "pr_opened": text = [data.branch, data.commit && shortOid(data.commit)].filter(Boolean).join(" "); break;
    case "pr_merged": text = say("merged_by", { reviewed: shortOid(data.reviewed_commit || "?"), commit: shortOid(data.commit || "?"), by: data.verified_by }); break;
    case "pr_rejected": text = say("rejected_by", { by: data.by, why: data.why }); break;
    case "approval_resolved": text = data.verdict; break;
    case "steer_received": text = data.text; break;
    case "gate_denied": case "budget_limit": case "provider_degraded": text = code(data.error ?? data); break;
    case "run_frozen": text = data.completion; break;
    default: text = "";
  }
  return clip(text === undefined || text === null ? "" : String(text), 200);
}
function clip(text, most) { return text.length > most ? text.slice(0, most - 1) + "…" : text; }

function policyText(policy) {
  return policy ? say("policy", policy) : say("policy_none");
}
function relatedText(related, key, fill) {
  if (!related) return null;
  if (related.run) return say(key, Object.assign({ run: short(related.run) }, fill));
  return related === "withheld" ? say("withheld_run") : say("missing_run");
}

// ---------------------------------------------------------------- drawing helpers

let M;
const state = { current: 0, axis: "order", speed: 1, playing: false, filter: "all", run: "", chapter: "", text: "", context: false, shown: PAGE };

function cite(seq) {
  const line = M.lines.get(seq);
  const outside = line && line.place === "context";
  return el("button", {
    type: "button", class: outside ? "cite outside" : "cite", "data-seq": seq,
    title: outside ? say("end_outside", { seq }) : say("end_at", { seq }),
    onclick: () => show(seq),
  }, "#" + seq);
}
function end(value) {
  if (value === "withheld") return el("span", { class: "end withheld" }, say("end_withheld"));
  if (value === "pending") return el("span", { class: "end pending" }, say("end_pending"));
  if (value === "missing") return el("span", { class: "end missing" }, say("end_missing"));
  if (seqIn(value, "at")) return cite(seqIn(value, "at"));
  if (seqIn(value, "outside")) return cite(seqIn(value, "outside"));
  return el("span", { class: "end missing" }, say("end_missing"));
}
function tag(flag) { return el("span", { class: "tag " + flag }, say("tag_" + flag)); }
function table(columns, rows, opts) {
  return el("table", { class: "table" + (opts && opts.stack ? " stack" : "") },
    el("thead", null, el("tr", null, columns.map((col) => el("th", { class: col.num ? "num" : null, scope: "col" }, say(col.key))))),
    el("tbody", null, rows));
}
function cell(col, content) {
  return el("td", { class: col.num ? "num" : col.nw ? "nw" : null, "data-label": say(col.key) }, content);
}
function runLabel(runId) {
  return runId === CITY_RUN ? say("city_lane") : laneName(runId) + " " + short(runId);
}
function laneName(runId) {
  if (runId === CITY_RUN) return say("city_lane");
  const run = M.runs.get(runId);
  return run && run.addr ? run.addr : say("no_addr");
}

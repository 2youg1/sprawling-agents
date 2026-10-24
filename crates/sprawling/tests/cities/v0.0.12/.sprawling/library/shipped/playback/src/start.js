// ---------------------------------------------------------------- start

function translateStatic() {
  document.querySelectorAll("[data-word]").forEach((node) => { node.textContent = say(node.getAttribute("data-word")); });
  document.querySelectorAll("[data-word-label]").forEach((node) => { node.setAttribute("aria-label", say(node.getAttribute("data-word-label"))); });
}

function start() {
  translateStatic();
  const read = readBundle();
  if (read.problem) {
    $("title").textContent = say("playback");
    $("subtitle").textContent = read.problem;
    document.querySelector("main").hidden = true;
    document.querySelector("nav.chapters").hidden = true;
    return;
  }
  M = model(read.bundle);
  drawHeader(); drawNarration(); drawRuns(); drawChapters(); drawLanes(); drawMoments(); drawCommits(); drawMessages(); drawCalls(); drawFilters(); drawLog(); drawCost(); drawWithheld(); drawSource();
  setCurrent(0);
  $("first").addEventListener("click", () => jump(false));
  $("last").addEventListener("click", () => jump(true));
  $("prev").addEventListener("click", () => step(-1));
  $("next").addEventListener("click", () => step(1));
  $("play").addEventListener("click", () => play(!state.playing));
  $("axis-order").addEventListener("click", () => { state.axis = "order"; drawAxisNote(); drawCard(); drawPlot(); });
  $("axis-time").addEventListener("click", () => { state.axis = "time"; if (M.events[state.current].timeIndex === undefined && M.timed.length) state.current = M.timed[0].index; drawAxisNote(); drawCard(); drawPlot(); });
  $("plot").addEventListener("click", plotClick);
  new ResizeObserver(() => drawPlot()).observe($("plot").parentElement);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => drawPlot());
  // The step keys belong to the timeline, so Home and End still scroll the page elsewhere.
  $("timeline").addEventListener("keydown", (event) => {
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || event.altKey || event.ctrlKey || event.metaKey) return;
    const keys = { ArrowRight: () => step(1), ArrowLeft: () => step(-1), Home: () => jump(false), End: () => jump(true), " ": () => play(!state.playing) };
    if (event.key === " " && target instanceof HTMLButtonElement) return;
    const act = keys[event.key];
    if (act) { event.preventDefault(); act(); }
  });
}
start();

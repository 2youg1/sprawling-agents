# skills

Agent skills that ship with sprawling. Each directory holds one skill, and its `SKILL.md` begins with the name, the description an agent reads to decide when to load it, and the licence.

| Skill | What it is for |
|---|---|
| [`authority-review`](authority-review/SKILL.md) | A two-pass audit of a branch before it merges: bugs and broken behaviour first, then every fact that has more than one definition |
| [`blast-radius`](blast-radius/SKILL.md) | What a change could break outside its diff, proved by running code |
| [`gauge`](gauge/SKILL.md) | Measuring a performance change with `sprawling gauge`: a pinned load, a baseline, alternating rounds, and a gate on a count rather than a clock |
| [`how`](how/SKILL.md) | How a subsystem works, and where a piece of code should live |
| [`playback`](playback/SKILL.md) | A stretch of a city's history made into one self-contained HTML page a person reviews offline, the reference page to start from, and the five checks it passes |
| [`sdd`](sdd/SKILL.md) | Specification-first development with Lean contracts |
| [`translation`](translation/SKILL.md) | Translating texts whose form carries the thought into Chinese |
| [`tutor`](tutor/SKILL.md) | Teaching one person a subject through dialogue until they can diagnose it |
| [`why`](why/SKILL.md) | Why something is the way it is, answered from every source of evidence that can be reached |

[`LICENSES.md`](LICENSES.md) gives each skill's licence and attribution. The release archive carries this directory exactly as it stands, this file included, because the archive walks it rather than listing skills (the `Skills` part in `tools/xtask/src/package/contents.rs`); a change here reaches people with the next release.

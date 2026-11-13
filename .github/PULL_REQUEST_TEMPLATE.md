<!--
  A pull request is judged by one question: does this change hold?
  The checklist below is the evidence a reviewer needs; the diff shows
  what you did, so the description should say what you found and why.
  Provide matching English and Chinese versions of the description,
  following AGENTS.md's Language section; share literal diagnostics,
  logs and attachments once.
-->

**What this is** — one sentence: the problem it closes or the capability it adds.

**What I found** — the finding worth writing down: a gate that changed the design, a red-to-green that exposed a real defect, a choice between two approaches whose reason is not visible from the result (CONTRIBUTING §5).

## Checklist

- [ ] **Affected local checks green on the final diff** — select them by impact under [verification.md](../docs/verification.md#tier-1--while-iterating), starting with the affected tests and gates and adding the behaviour checks they do not cover; list each command and its result. `just check` is required only when that policy selects it:
- [ ] **Branch `ci` green before opening this PR** — link the exact run on the same branch. A contributor without push access runs it in a fork first (CONTRIBUTING §4.1):
- [ ] **Gate machinery is separate** — a path classified by [gates.md](../docs/gates.md) sits in a commit apart from the source it judges. If the change loosens a gate in the change that gate rejects, the commit carries `Verdict: user-approved`; state the ruling here:
- [ ] **SPEC in step with the code** — the crate's SPEC (`Spec.lean` and its parts, or `<crate>-SPEC.md` until it migrates) updated in the same change-set as any interface change it describes.
- [ ] **Module map in step** — a new production module registered in `architecture.toml` before or with it (`xtask modmap`); a test module needs no entry.
- [ ] **Red-to-green visible** — for a defect fix, the failing test sits in the history before the fix (ub-mode §3).
- [ ] **Nothing private in this description or in the commit bodies** — no credential, no absolute home path, no pasted terminal dump, no conversation or tool-call log, no personal or machine name, no uncropped screenshot. Reread once before opening: **no gate scans these** (CONTRIBUTING *Public content and privacy*), and a later force-push does not remove what was already fetched.

## Notes

- `deps:` is the bot's family and carries no card number (CONTRIBUTING §5); a person's change uses `CS<stage>.<index>: <what>`.
- A change that widens a gate instead of fixing the cause is the one thing `guard` exists to refuse; if you believe a gate must move, say why here rather than in the code.

# v0.0.5 — pre-alpha

The shape of the work was: make the first ten minutes work, and make the
screen a person judges this product by say what it means.

### A model that was never asked to answer

Every model the built-in catalogue did not know was registered with an
output ceiling of zero, and zero was sent on the wire. A provider answers
`max_tokens: 0` with no content at all, the run then froze as `done`, and
the thread showed a run that thought for three and a half minutes and
said nothing. Two independent paths produced that false history, so both
are closed.

- `kernel::Ceiling` wraps a non-zero count, and a model's ceiling is
  `Option<Ceiling>` from the catalogue row to the wire: absent is absent,
  and zero cannot be spelled. The OpenAI wire omits the field and takes
  the provider's default; the Anthropic wire, which requires it, refuses
  with a recovery naming where to register the number.
- A reply with nothing in it, or one the provider cut off at the ceiling,
  freezes as `limit` rather than `done`. `Completion::Done` has to cite a
  `model_returned` as its evidence, and an empty one is not evidence.
- Usage is asked for on OpenAI-shaped streams, which never reported it
  before, so every streamed call billed something and accounted nothing.

### The machine's own proxy, and never for itself

reqwest was built with default features off, which dropped the system
proxy along with them: on Windows and macOS this was the one tool on the
machine that could not see the proxy its owner had configured, and the
symptom was a fifteen-second timeout naming nothing. SOCKS comes with it
and costs no package. Found by running the suite afterwards: with a
system proxy compiled in, a machine whose owner runs one sent loopback
through it, and a local model server answered 502 through somebody else's
gateway — so every client that can reach this machine refuses to proxy
it. `kernel::reach` says where a call stops, stage by stage, so a refusal
can name the stage instead of quoting an error chain.

### The screens

One implementation per control instead of a class string per view. The
composer opens in the middle of an empty room and rides down to the bar
on the first send. A `/` in the box opens the same command table the
palette reads. Keys are data: an action table, platform rendering,
rebinding and conflict detection. The provider form can hold a second
key — the reference now carries the name it was minted from, which is
what let the first provider's key reach the second. A model table can
state the ceilings the wire could not carry before. A browser is a
family rather than a brand, so a machine with Zen or Brave on it is
ready. Both install scripts show version, platform, percent and the
checksum result. Paths a page prints can be opened where a person keeps
their files.

Geist Sans and Geist Mono ship with the client, which reverses a recorded
decision: a type stack that begins with whatever the machine happens to
have is a different product on every machine.

### Numbers

- The client the browser downloads: 282.1 KiB gzipped, half of it the two
  faces. The register was re-priced with that reason.
- A durable write costs one disk barrier, and a barrier costs the same
  for fifty records as for one: 585 microseconds per record at one per
  barrier against 13 at fifty on one windows-x86_64 NVMe machine, of
  which about 2.5 is the write. The ledger port grew a batch face and the
  relay hands over everything already waiting at once.
- npm's platform packages moved into the `@sprawling` scope; the root
  package keeps its bare name, because `bunx sprawling` is what the
  channel exists for.

## Please do not download this release

**It is published to test the release pipeline, not to be used.** Treat
anything it produces as scaffolding. If you want to look at the project,
read the source. If you want to run something, wait for a release that
does not carry this notice.

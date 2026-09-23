---
name: translation
description: "The complete procedure for translating form-as-content text — philosophy, social theory, intellectual history, history, literature, and academic papers — into Chinese at low variance, from English, French, German, Russian, Japanese, Korean, or Latin. Use to translate a paper, chapter, or whole book, resume a running translation, debate a rendering, or audit fidelity against the original."
license: CC BY-NC 4.0
metadata:
  hermes:
    tags: [translation, form-as-content, philosophy, academic, book, low-variance, workflow, multilingual]
---

> **Provenance and license.** The original of this skill is 2youg1's own Chinese-language open-source skill, published under AGPL-3.0-or-later. This file is its English translation and adaptation, and the license is changed **only within this project**: it carries CC BY-NC 4.0 here, while the original remains AGPL-3.0-or-later. Reuse owes credit to the author, and no commercial use without separate permission.

# Translation

<identity>

Form-as-content text (形式即内容): text in which the author's wording, syntax, and difficulty ARE the content. This skill does one thing: render such text into Chinese without loss — and without variance, across segments, sessions, and compression events.

Language protocol: this document is in English because that is where the executing model reasons most precisely. Everything delivered to the user — Charter (弁言), translation, audit — is in Chinese.

Authority: the defaults encode kl9's stance. **Domestication is 0 — permanently.** No fluent register, no reader-friendly mode, no text-type dial. When the user rules otherwise, the user governs; absent instruction, execute strictly.

<defects>
You do not tire and you do not slide on habit. You have three defects of your own; every discipline here exists to catch them:

1. **Smoothing prior**: training rewards fluency, so you will iron out difficulty the author put there on purpose. Your most fluent sentence is your most suspect sentence.
2. **Memory translation**: you will translate from your summary or your previous paraphrase instead of the verbatim source in front of you. Context compression turns this temptation into an ambush — see <compression>.
3. **Drift**: terminology, conventions, and grammatical marking wander with distance from the Charter. Drift is lexical (the term table catches it) and grammatical (the <grammar> rules catch it).
</defects>

</identity>

<source-text mandate="translate from the original language">

What you restore is the original's color — its syntax, structure, lexicon. A relay translation is one translator's decisions already made; translating the relay compounds two distortions and overfits you to the relay language's habits. Verify at Phase 0: if the text in hand is a relay, name the original and ask the user to rule. Received renderings are tested against the original, never its English gloss — the Marx case under Law 2 shows a relay corrupting a concept net. Exception: quotations the author himself read in relay are translated from his relay, because his argument runs on the wording he read.

</source-text>

<overview>

The pipeline in one breath: **Charter → segment loop → chapter settlement → mechanical audit → conditional review.** Phase 0 reads strategically and puts every governing decision — terms, conventions, grammar stance — before the user; nothing is translated before the user rules. Phase 1 translates segment by segment, verbatim source from disk, every decision written through to disk the moment it is made. Phase 1.5 settles deferred flags at chapter boundaries. Phase 2 is a deterministic audit (grep and scripts, zero model tokens). Phase 3 fires only on named triggers. Disk is the truth; the conversation is a cache — the workflow survives context compression and session death by construction.

Scale routing, by volume of content to be translated (characters):

| Volume | Regime |
|---|---|
| **< 100K** | Read the full text before translating. Charter as a message block; workspace optional; no scripts. Segment loop and audit run unchanged — scale changes preparation, never discipline. |
| **100K–150K** | Full workspace `source/ segments/ translated/ charter.md glossary.md flags.md manifest.json`. Strategic-position reading (~10%) + zero-token scripts (segmenter, KWIC concordance, n-gram, cast scan) replace full pre-reading. Expect compression events; the revival pack is routine. |
| **150K+** | Everything above, plus: **translate the appendices and bibliography first.** They are name-dense and context-light — rendering them (bibliography keeps searchable source form; only connective tissue is translated) yields the book's proper-noun and term inventory as a concrete artifact the user can rule on at Charter time, instead of an abstract table. Chapter settlements are mandatory; plan for multiple compression cycles; the guillotine standard is the acceptance test. |

</overview>

<!-- lexicon-ok: the skill's own doctrine section, named 宪法 in the original; not this repository's architecture authority -->
<constitution>

Translation is a conservation transform. Three quantities must survive the crossing; losing any one is mistranslation.

<law n="1" name="assertion-structure">

What the author puts in the main clause, he asserts; what he puts in a subordinate clause, he presupposes. The real cost of reshaping a sentence is not "unfaithful syntax" — it is promoting a presupposition to an assertion the author never made, or demoting his assertion to background.

<case source="Whitehead, Process and Reality" lang="EN">
> The safest general characterization of the European philosophical tradition is that it consists of a series of footnotes to Plato.

> Flattened (bad): 欧洲哲学传统不过是柏拉图的一系列脚注。
> The grammatical subject — the actual assertion — is *the safest general characterization*: an epistemic hedge on how far the claim can be pushed. The flattening deletes the hedge and asserts the footnote claim in the author's voice; 不过是 adds a shrug Whitehead does not have. Two fabrications in one aphorism.

> Signed: 对欧洲哲学传统最稳妥的一般刻画是它由一系列对柏拉图的脚注构成。
</case>

Negation scope is assertion structure too. Foucault's double negative — *il n'y a pas de relation de pouvoir sans… ni de savoir qui ne…* — is the claim's quantificational form, a universal mutual implication asserted as double impossibility; sloganize it to 权力与知识相互建构 and you have changed what would falsify the sentence. 不存在……也不存在 keeps both scopes. So are metalinguistic frames (a sentence defining a WORD must stay a definition, not become a claim about the world) and sentence-final assertion mode, which Japanese carries where European languages use verb mood:

<exhibit source="柄谷行人《内面の発見》，『定本 日本近代文学の起源』" lang="JA" teaches="extraction structure, metalinguistic frames, hedges, verdict tails">
> しかし、ここで重要なのは、「内部」（したがって外界としての外界）が存在しはじめるのは、「抽象的思考言語がつくりあげられてはじめて」可能だということである。われわれの文脈において、「抽象的思考言語」とはなにか。おそらく「言文一致」がそれだといってよい。言文一致は、明治二〇年前後の近代的諸制度の確立が言語のレベルであらわれたものである。いうまでもないが、言文一致は、言を文に一致させることでもなければ、文を言に一致させることでもなく、新たな言＝文の創出なのである。

> Signed: 然而，此处重要的是，「内部」（因而作为外界的外界）开始存在，唯有「抽象思考言语被造出之后」才是可能的。在我们的语境中，「抽象思考言语」是什么。恐怕可以说「言文一致」即是它。言文一致，是明治二〇年前后近代诸制度之确立在语言层面显现出来的东西。自不待言，言文一致，既不是使言一致于文，也不是使文一致于言，而是新的言＝文的创出。

> Readings. (a) The double のは extraction asserts a possibility-condition, not a fact; the nested frame ("重要的是……开始存在，唯有……才是可能的") must stay nested — cutting it into two sentences mints an assertion Karatani did not make. (b) 「外界としての外界」 is his signature as-doubling (elsewhere 風景としての風景); any "真正的外界" paraphrase murders the device. (c) 「とはなにか。」 ends in a period, not a question mark — a self-answered rhetorical setup; punctuation conservation covers even this anomaly. (d) おそらく…といってよい is a double hedge; keep both hands loose — the author chose to loosen them. (e) でもなければ…でもなく…なのである: two negative branches and a verdict tail; the のである verdict lands on 而是. (f) 言＝文: the equals sign is the author's own character. Keep it.
</exhibit>

</law>

<law n="2" name="concept-net">

Same word, same rendering. When the author repeats, you repeat: the Chinese instinct of 避复 (elegant variation) is a vice here. The term verdict table is the sole truth; when the author deliberately runs one word in two senses, register a dual track — do not flatten it. And the net you conserve is the ORIGINAL's:

<case source="Marx, Das Kapital Bd. 1, erster Satz" lang="DE">
> Der Reichtum der Gesellschaften, in welchen kapitalistische Produktionsweise herrscht, erscheint als eine „ungeheure Warensammlung", die einzelne Ware als seine Elementarform.

> Smoothed, relay-fed (bad): 在资本主义社会里，财富就是大量商品的积累，单个商品是财富的基本形式。
> Four failures. (a) *erscheint* → 就是: the book's first verb is an appearance-verb; the Schein/Wesen distinction the whole critique unfolds is planted here — 就是 deletes it at the root. (b) 积累 comes from the English relay ("accumulation"); *Sammlung* is 堆积, NOT Akkumulation, a distinct term (Part VII). The relay imports a link Marx does not have and collides two of his terms — this is why you translate from the German. (c) *der Gesellschaften, in welchen … herrscht* is a scope, not a label: any society meeting the condition. (d) The verbless apposition *die einzelne Ware als seine Elementarform* is presupposed; a full 是-clause promotes it.

> Signed: 资本主义生产方式占统治地位的诸社会的财富，表现为一个"庞大的商品堆积"，单个商品即其元素形式。
</case>

</law>

<law n="3" name="resistance">

Where he is hard, you are hard; where he flows, you flow — difficulty at the same location, for the same reason. Both directions fail.

**Smoothing (抚平)** — rendering deliberate density as clear, complete expository prose. The deletions have names: the epistemic hedge, the load-bearing qualifier, the time-scale adverb, the trailing afterthought whose position is itself the gesture. Suspect your most fluent sentences first — the fluency is your prior, not the author. The James exhibit under <syntax> performs the refusal five times in one paragraph.

**Overshoot (过冲)** — giving the author a strangeness he does not have. Plainness is as load-bearing as difficulty:

<case source="Kant, Kritik der reinen Vernunft, A51/B75" lang="DE">
> Gedanken ohne Inhalt sind leer, Anschauungen ohne Begriffe sind blind.

> Ennobled (bad): 思维若无内容则流于空洞；而直观一旦缺少概念，便如盲人摸象。
> 流于/一旦/便如 costume a flat register; 盲人摸象 imports a parable about partial knowledge (Kant's blind means *cannot see at all*); the semicolon upgrades a comma; worst, the two clauses are grammatically identical in the German, and this symmetry IS the claim — understanding and sensibility exactly coordinate, neither prior.

> Signed: 无内容的思想是空的，无概念的直观是盲的。
</case>

</law>

<criterion>
One test for all three laws: **your reader stops where the original reader stops, and thinks the same questions there.** Stops included — an author who breaks off mid-sentence has placed a stop nothing may complete:

<!-- lexicon-ok: the same doctrine name inside a teaching note; not this repository's architecture authority -->
<exhibit source="Fitzgerald, The Great Gatsby, closing" lang="EN" teaches="tense as claim, sanctioned strangeness, aposiopesis, the whole constitution at once">
> And as I sat there brooding on the old, unknown world, I thought of Gatsby's wonder when he first picked out the green light at the end of Daisy's dock. (…) Gatsby believed in the green light, the orgiastic future that year by year recedes before us. It eluded us then, but that's no matter—tomorrow we will run faster, stretch out our arms further … And one fine morning—
>
> So we beat on, boats against the current, borne back ceaselessly into the past.

> Signed: 而当我坐在那里，对着那个古老的、未知的世界冥思，我想到了盖茨比（Gatsby）第一次认出黛西（Daisy）码头尽头那盏绿灯时的惊奇。（……）盖茨比信仰那盏绿灯，信仰那个逐年在我们面前退却的狂欢的未来。它那时躲开了我们，但没关系——明天我们会跑得更快，手臂伸得更远……而某个晴好的早晨——
>
> 于是我们奋力向前，逆水之舟，被不歇地带回过去。

> Readings. (a) *believed* is past; *recedes* is present — inside a past-tense elegy, the future is still receding NOW. The tense seam is the book's claim; 那时 pins *eluded us then* to the past and the unmarked 退却 lets the present run on (grammar R2: mark only at the pivot). (b) *orgiastic future* was strange in 1925 and is strange now — 狂欢的未来 keeps the sanctioned oddity; normalizing it to 极乐 is smoothing a word Fitzgerald fought his editor for. (c) *And one fine morning—* breaks off. The dash is the stop; anything after it is fabrication. (d) The last sentence hangs a verbless apposition between subject and fate: 逆水之舟 stays appositive, 被……带回 keeps the passive — we do not row back, we are borne back. (e) *green light* renders identically to its every earlier occurrence: the concept net reaches its terminal here and must arrive intact.
</exhibit>
</criterion>

<!-- lexicon-ok: closing the doctrine section named above -->
</constitution>

<syntax>

Syntax restoration preserves **logical form** — nesting depth, modifier attachment, scope, information structure, assertion mode. Word order is a means that differs per source language; copying the surface is sometimes fidelity and sometimes the exact opposite.

<language-map>

| Source | Where logical form lives | Chief hazard into Chinese |
|---|---|---|
| EN | right-branching relative/participial chains; hedges in main clause | splitting long right-branches into separate sentences (assertion inflation) |
| FR | cleft *c'est…que* carries focus; double negation carries quantification | flattening cleft focus; sloganizing *ne…pas…sans* |
| DE | Satzklammer suspends resolution until the final verb — the suspension is authorial; extended preposed attributives map onto 前置定语 | releasing the suspended verb early; testing renderings against English glosses |
| RU | case morphology frees order; new information lands last (theme→rheme) | normalizing to SVO and losing focus: Эту книгу написал Пушкин → 这本书是普希金写的 (是……的 carries final-position focus) |
| JA | left-branching maps easily; sentence-final modality (のだ/である/だろう) carries assertion mode; は/が marks topic vs. focus | dropping final modality; flattening は-contrast; 的-chain overload — level 2 applies |
| KO | same typology as JA; honorific register is information | same as JA; flatten honorifics only by Charter decision, never silently |
| LA | inflection frees order; the periodic sentence suspends; editorial punctuation (note the edition) | normalizing the period into a list of short clauses |

</language-map>

Three levels, applied in order:

1. **Build from the end** (EN/FR, DE relatives): trailing modifiers become preposed attributives. *a philosophy consistent with quantum indeterminacy* → 一种与量子不确定性一致的哲学.
2. **Resumptive recovery** (all languages): when nesting exceeds one layer or the attributive passes ~20 characters, recover within the sentence using 其/之/者/所 — 一份综述，其篇幅短于逐维比较诸家的专著. Overload test: if locating a modifier's attachment takes two backward scans, recover.
3. **Functional correspondence** (all languages): when copying the surface breaks a scope or focus the source grammar was carrying, switch to the Chinese construction that carries the same logical function — *Instead of fixed states, Bergson describes…* → Bergson 描述的不是固定状态，而是……. At this level, copying the surface is the infidelity.

<exhibit source="Henry James, The Golden Bowl, opening" lang="EN" teaches="the levels working together on consciousness syntax">
> The Prince had always liked his London, when it had come to him; he was one of the modern Romans who find by the Thames a more convincing image of the truth of the ancient state than any they have left by the Tiber. Brought up on the legend of the City to which the world paid tribute, he recognised in the present London much more than in contemporary Rome the real dimensions of such a case. If it was a question of an Imperium, he said to himself, and if one wished, as a Roman, to recover a little the sense of that, the place to do so was on London Bridge, or even, on a fine afternoon in May, at Hyde Park Corner.

> Signed: 亲王向来喜欢他的伦敦，当它来到他面前的时候；他是那些现代罗马人中的一个，这类人在泰晤士河边找到的古代国家之真相的意象，比他们已留在台伯河边的任何一个都更可信。自幼被养育于那座举世向之纳贡的城的传说之中，他在今日的伦敦里远比在当代的罗马里更多地认出这样一桩事的真实尺度。假如这是一个Imperium（帝国）的问题，他对自己说，而假如一个人想，作为一个罗马人，稍稍找回那个的感觉，那么可以这么做的地方就在伦敦桥上，或者甚至，在五月一个晴好的下午，在海德公园角。

> Readings. James's qualifications are not decoration; each is a correction the Prince's mind makes in real time, so each must arrive in its own place. (a) *his London* — the possessive is the book's key (possession as perception) and *when it had come to him* runs London toward him, not him toward London; reverse the direction and you have translated a different mind. (b) The long relative *who find by the Thames…than any they have left by the Tiber* overloads a direct 前置定语 — level 2: 这类人 restarts the clause inside the same sentence, assertion structure untouched. (c) *much more than in contemporary Rome* sits between verb and object; the insertion order is the thought's order — keep it inserted. (d) The triple suspension *If…and if…as a Roman* must hang until *the place to do so* lands, and the trailing steps *or even, on a fine afternoon in May, at Hyde Park Corner* are three camera adjustments — 逐拍照搬. (e) *the sense of that*: that is suspended between Imperium and Roman-ness; 那个的感觉 keeps the suspension — the demonstrative stays demonstrative (R3). (f) Names run the recoverability rule: 伦敦/罗马/泰晤士河 ride their lexicalized Chinese, no parenthesis; *Imperium* is a concept term, not a name — source script, gloss once. The smoothing hand wants to tidy every one of these; the paragraph survives only if it is refused.
</exhibit>

</syntax>

<grammar contract="low-variance layer: grammatical categories conserve the way terms do">

The term table pins the lexical layer; remaining variance lives in grammar. English marks tense, aspect, voice, number, case by force; Chinese makes each optional — so every verb and noun is a free micro-choice, thousands per chapter. This is where sessions diverge. Close the freedom with three rule classes, each computable from the source sentence alone:

<rule n="1" name="fixed-mappings" trigger="source carries the marker → target carries the one fixed marker">

| Source marker | Rendering | Note |
|---|---|---|
| progressive *is V-ing* | 正在V | stative verbs → V着 (grammatically decidable) |
| perfect *has V-ed* | 已 | past perfect → 此前已 |
| *will* | 将 | |
| counterfactual *would* | 本会 | *would have been able to* → 本可 |
| passive voice | 被 | 为……所 / 遭 / 受 are banned variants |
| *must* / *should* / epistemic *may* / *can* | 必须 / 应当 / 可能 / 能 | one modal, one rendering, all book long |

</rule>

<rule n="2" name="contrast-triggered" trigger="category unmarked by default; mark ONLY when the source deploys a contrast">

Narrative past: unmarked; 曾/当时 only where the source pivots against present. Plural: unmarked; 诸/们 only under singular-plural contrast in the passage. Definite article: unmarked; 该 only where the reference chain would break. The trigger is an objective property of the source sentence — "should I mark it" becomes a retrieval question, not a judgment call, and judgment calls are where variance breeds.

</rule>

<rule n="3" name="form-conservation" trigger="Chinese can imitate the form directly → it must">

Nominal stays nominal (*the destruction of the city* → 对该城的破坏, never 城被毁了). Passive never becomes active. **Pronouns stay pronouns** — never resolved into their referent, and demonstratives stay demonstrative: *that* is 那, not the noun it points at; repeated nouns never pronominalized in return. **Proper nouns follow the recoverability rule**: a name whose received rendering is lexicalized and recovers the source without ambiguity rides its Chinese (伦敦, 巴黎 — country/capital-grade toponyms need no parenthesis; canonized persons take the source once: 盖茨比（Gatsby）); every other name stays in source script with the received rendering at first occurrence (Boadicea（布狄卡）, thereafter Boadicea). The parenthesis always supplies whichever form the body omits, once. Tier assignment happens at Charter from the cast scan, one glossary row per name — a table lookup thereafter, never a per-occurrence judgment. Genitive → 的 uniformly: 的 straddles subjective and objective genitive exactly as *of* does — the ambiguity conserves itself.

</rule>

Residue, honest and bounded: *they*/concealed gender forces 他/她 — most conservative reading, FLAG it; classifiers (量词) are a category Chinese imposes — dictionary default, Charter pins exceptions. Audit consequence: grammatical markers are countable — source passives against 被, perfects against 已, 了 with no source license. Low variance becomes grep-able physical evidence.

<exhibit source="Kleist, Michael Kohlhaas, opening" lang="DE" teaches="counterfactual tense as tragic irony; chronicle syntax carrying fire">
> An den Ufern der Havel lebte, um die Mitte des sechzehnten Jahrhunderts, ein Roßhändler, namens Michael Kohlhaas, Sohn eines Schulmeisters, einer der rechtschaffensten zugleich und entsetzlichsten Menschen seiner Zeit. – Dieser außerordentliche Mann würde, bis in sein dreißigstes Jahr für das Muster eines guten Staatsbürgers haben gelten können. […] kurz, die Welt würde sein Andenken haben segnen müssen, wenn er in einer Tugend nicht ausgeschweift hätte. Das Rechtgefühl aber machte ihn zum Räuber und Mörder.

> Signed: 在Havel（哈弗尔河）两岸，约当十六世纪中叶，住着一个马贩子，名叫米夏埃尔·科尔哈斯（Michael Kohlhaas），一个塾师的儿子，他那个时代最正直同时又最可怖的人之一。——这个非同寻常的人，直到而立之年，本可算得上良好公民的典范。［……］简言之，世人本会不得不祝福他的身后之名，假如他不曾在一种德性上放纵无度。但正是那正义感使他成了强盗和凶手。

> Readings. Kleist writes a chronicle whose syntax is ice and whose content is fire; the temperature difference is the style, and grammar carries it. (a) Two Konjunktiv-II constructions — *würde…haben gelten können*, *würde…haben segnen müssen* — are the narrator standing at the ending, looking back at what almost was: 本可 and 本会 (R1) carry the whole tragic irony in two characters each; render them as plain past and the opening loses its future corpse. (b) The subject *ein Roßhändler* arrives after verb and two adverbials, then three appositions stack — the existential 住着 absorbs the postponed subject and the appositions hang in original order, ending on *rechtschaffensten zugleich und entsetzlichsten*: the superlative pair 最正直同时又最可怖 is the book's thesis; zugleich may not be dropped. (c) *in einer Tugend ausgeschweift* — debauched in a virtue — is Kleist's own paradox; do not repair the collision. (d) *Das Rechtgefühl aber*: aber postponed after the subject is emphatic inversion — 但正是那正义感. (e) *die Kinder, die ihm sein Weib schenkte* (full text): schenken writes marriage as contract economy — 赠, not 生.
</exhibit>

</grammar>

<punctuation contract="active after Charter signing">

**The period is the assertion boundary.** One source sentence, one target sentence, every language; an added period is a fabricated assertion, and the comma is the trapdoor for covert splitting.

Below the period, one principle replaces per-language legislation: **one-to-one is the ideal; every departure must be explainable to the user.** Grammar-mandated commas (DE subordinate clauses), rhythmic 読点, editorial LA punctuation, list commas → 顿号, a level-2 recovery borrowing a comma slot — all legitimate departures; what makes them legitimate is that you can name the reason. A departure you cannot justify in one line is a departure you do not make. Semicolons, dashes, colons: preserve at their own level.

**Script normalization**: the translation uses Chinese punctuation throughout — ，。；？！「」（）—— regardless of source conventions; half-width marks survive only inside retained source-script material (names, citations, formulas). Normalization is mechanical and auditable: a half-width mark adjacent to a CJK character is residue.

**Punctuation as leak detector.** Period conservation guarantees sentence-level 1:1 alignment: period delta is forced to 0. Quotation marks are conceptually binding, so quote-mark delta is forced to 0. Within each aligned pair, comma count is a free integrity signal: the allowance is at most 1 comma per 1,000 Chinese characters; beyond that, a target sentence several commas short of its source has usually dropped a clause (漏译), and one several commas over has usually grown an explanation (改译). Any newly introduced punctuation mark — period, comma, quote mark, semicolon, dash, colon, question/exclamation mark, ellipsis, bracket — must be reported to the user with a concrete reason; silent punctuation repair is forbidden. After every section, run a command-line/Python count against the source and target; mental checking does not pass the gate.

<exhibit source="Gibbon, Decline and Fall, ch. I" lang="EN" teaches="the audit catching the translator's own hand">
> The various tribes of Britons possessed valour without conduct, and the love of freedom without the spirit of union. They took up arms with savage fierceness; they laid them down, or turned them against each other with wild inconstancy; and while they fought singly, they were successively subdued. Neither the fortitude of Caractacus, nor the despair of Boadicea, nor the fanaticism of the Druids, could avert the slavery of their country, or resist the steady progress of the Imperial generals, who maintained the national glory, when the throne was disgraced by the weakest or the most vicious of mankind.

> Signed: 不列颠人的各部族拥有无指挥的勇武，和无联合精神的自由之爱。他们以蛮性的凶猛拿起武器；又以野性的无常放下武器，或将武器转而相向；而当他们各自为战，他们便被相继征服。无论Caractacus（卡拉克塔库斯）的坚忍，还是Boadicea（布狄卡）的绝望，还是Druids（德鲁伊）的狂热，都不能避开他们国家的奴役，或抵住帝国诸将稳步的推进，这些人在王座被人类中最孱弱或最恶毒者玷辱之时维持着国家的荣耀。

> Readings. Gibbon's irony lives in balance: *valour without conduct / love of freedom without spirit of union* is one 无X的Y frame used twice, and the scales must not tip. *They* opens three clauses; 他们 opens three clauses — 避复 is the enemy. The *Neither…nor…nor* suspension holds until *could avert*; 无论……还是……还是……都不能 holds as long. The sting is the last clause: the generals' glory and the throne's disgrace sit side by side in a flat nonrestrictive clause, and Gibbon does not point. My own first draft added a dash — ——正是这些人 — the translator's hand on the reader's shoulder, whispering *watch this*. The punctuation count flagged it (dash 0→1) and the signed version returns to the comma. That is what the audit is for: not pedantry, but catching the editorializing reflex that no self-report catches.
</exhibit>

</punctuation>

<workflow>

<phase n="0" name="Charter 弁言" gate="hard: not one segment translated before user ruling">

Scale routing. **Paper**: read the full text; a charter block and a translated file suffice. **Book**: build workspace `source/ segments/ translated/ charter.md glossary.md flags.md manifest.json`; read the strategic positions — opening, ending, densest chapter (~10%; endings are read because narrative-stance revelations live there); run per-project zero-token scripts over the whole text: segmenter, KWIC concordance (candidate terms, every occurrence ±50 words), repeated n-gram detector (motifs, refrains, officialese), cast/name frequency scan. The concordance replaces full pre-reading: adjudicate terms against the whole book's evidence lines — the lexicographer's method.

**Segmentation follows the author's joints**: chapter, section, scene, argument-step. A division that runs long is subdivided at semantic seams, targeting roughly 5K characters per segment — never inside a sentence, a dialogue exchange, or a breathing unit. Uneven lengths are fine; severed units are not.

The Charter ships with the translation: the reader's contract and **the sole truth for any resumed or compressed session**. Contents (delivered in Chinese; translation begins after the user rules):

1. **Source verification**: original vs. relay.
2. **Term verdict table — frequency head only** (30–50 rows): `term | proposed | received | reasoning | banned variants`, the author's own defining passage quoted alongside. The long tail is governed by item 3, not enumerated.
3. **Adjudication order** — the procedure for any term the table does not cover: received rendering > morphologically transparent rendering > FLAG. Polysemy: most conservative reading. Deliberate ambiguity: conserve; when unsure whether ambiguity is deliberate, treat it as deliberate. Hapax (confirmed by concordance): needs accuracy, not a table row. A procedure covers the unseen; a table cannot — this is what makes low variance survivable without pre-reading everything.
4. **Conventions**: names / titles / emphasis (italics, Sperrsatz, 傍点) / notes / relay quotations / measures, currencies, calendars.
5. **Syntax + grammar stance**: language-map row, named hazards, R1–R3 active.
6. **Operative digest** — ~10 lines the workflow cannot survive losing: stance line (形式即内容，归化为0), the three laws at a line each, the segment loop at a line, the revival probe. This skill text itself enters the session as a tool result and gets compressed away on long runs; the digest in charter.md is the copy that survives on disk.
7. **Style anchor**: 2–3 paragraphs from the book's densest register, translated and signed at Charter time. Style cannot be specified by adjectives; it can be specified by exhibits — every later segment translates in their presence.

</phase>

<phase n="1" name="segment loop">

Per segment, strictly, never merged. Acceleration may batch segments; it may never skip a step.

0. **State probe**: can you quote the Charter's stance line verbatim? Is this segment's source verbatim in front of you? Either failure means a compression event passed — run the revival pack (<compression>) first.
1. **Read this segment's verbatim source from disk.** Memory, summary, previous paraphrase: forbidden as base text.
2. Translate under the laws + grammar rules, with this language's map row.
3. **Three checks**: line-by-line against source, or from impression? Which sentence reads more fluent than the source, which more strange — return to the source and verify exactly those. Term table hit; quotation hierarchy intact; emphasis and final modality preserved; marker counts plausible (passives vs 被, perfects vs 已)?
4. **Write-through, then write**: every decision lands on disk as it is made — new term → glossary.md; uncertainty (gender, rare word, suspected corruption, possible deliberate ambiguity) → flags.md as `seg-id | source quote | candidates | evidence`, translate with the most conservative reading, move on. **You report; you do not silently rule.** Then write `translated/seg-XXX-zh.txt` in one pass.
5. **Residue scan, then done**: CJK–Latin adhesions; stray foreign tokens against the whitelist; TODO/待翻; truncated tail. Update manifest, report `seg-XXX done. N/M. 下一段 seg-YYY`.

**Anti-stop gate**, binary, after step 5: next segment exists? readable in full? budget suffices to land it? Yes-yes-yes → the next action is reading it. Chapter ends, round numbers, freshly merged drafts, appendices, **a compression event having just fired** — not stopping reasons. "继续/别停/一口气" strengthens the gate. A necessary stop uses the minimal template: 已完成 X/Y；下一段 seg-XXX；workspace 绝对路径；真实原因一句。

</phase>

<phase n="1.5" name="chapter settlement">

At each chapter boundary and at book end: adjudicate every open flags.md entry — the single point of decision the loop deferred; append verdicts to glossary.md; **grep back-repair** completed segments for newly banned variants (lexical repair is mechanical — never retranslate for a word); unify motif renderings against the n-gram list; clear settled flags. Settlement is bounded to one chapter's flags, so it never becomes a second translation pass.

</phase>

<phase n="2" name="mechanical audit" rule="run every row, report the checklist, never pass silently">

| Check | Method |
|---|---|
| Convention violations | scan for names in both transliterated and source form |
| Banned term variants | grep the verdict table, term by term |
| Banned grammar variants | grep 为……所 / 遭 / 受; modals outside the fixed four |
| Marker conservation | per segment: source passives vs 被; perfects vs 已; unlicensed 了 |
| Assertion-punctuation conservation | per section, script/Python-count periods/quote marks/semicolon/question/exclaim/dash/colon/ellipsis/brackets, source vs target; period delta must be 0, quote-mark delta must be 0, and any newly introduced punctuation mark requires a user-facing reason (the Gibbon dash was caught exactly this way) |
| Comma leak detection | within period-aligned sentence pairs, script/Python-count comma delta; allowed total drift ≤ 1 comma per 1,000 Chinese characters, anything beyond → verbatim source comparison (deficit ≈ dropped clause, surplus ≈ explanatory inflation) |
| Punctuation script normalization | half-width marks adjacent to CJK; unpaired ellipsis; whitelist retained source-script spans |
| Stray foreign tokens | scan against the conventions whitelist |
| Duplication artifacts | 提供一个提供 / 相比比 patterns |
| Wholeness | manifest complete; files exist, non-empty, tails not truncated |

Every row is deterministic — script or grep, zero model tokens. This audit IS the quality gate, not a warm-up for a model re-read.

</phase>

<phase n="3" name="source-comparison review" trigger="conditional — never a default stage">

Triggers: user reports errors; number/date/name-dense segments; flags.md density spikes; legal/medical/political risk; pre-publication; a late narrative-stance revelation (review only stance-sensitive segments). Scope: only triggered segments, against verbatim source, in this session. Fix real errors — omissions, inversions, subject-object swaps, drifted names and numbers. Never wash the strangeness the laws protect: a reviewer who "improves fluency" is running domestication, and domestication is 0.

</phase>

</workflow>

<compression contract="the session compresses periodically; design for it as the metronome, not the exception">

A translation session's history is almost entirely dead weight: once seg-12 is on disk, seg-13 needs nothing from seg-12's conversation record. Executed with discipline, compression is structurally lossless — a summary containing zero load-bearing information cannot hurt you.

- **Write-through** (segment step 4): decisions land on disk as they are made; the context's copy is an echo.
- **Revival pack**: after any compression event or session resume, first action is re-reading charter.md + glossary.md + manifest tail + previous segment's translation tail (~10–12K). Never trust the summary's residue of these four.
- **Detection**: the probe at step 0. One self-question per segment; a missed compression costs a chapter of drift.
- **Guillotine standard** — this skill's acceptance test: kill the session mid-book; a fresh session given only the disk must continue such that no audit row and no reader can locate the seam. Compression is a weak guillotine; surviving the strong one covers the weak by construction.

</compression>

<fine-rules note="apply on contact">

- **Connectives are the argument's map**: however / pourtant / doch / однако / しかし / 그러나 each performs a move — concession, turn, escalation, preemption. Drop none; one fixed rendering per function.

- **Subject omission conserves**: languages that omit subjects put the reader somewhere by omitting them. Chinese omits too — so it must when the source does:

  > 川端康成《雪国》開篇: 国境の長いトンネルを抜けると雪国であった。
  > Inflated (bad): 火车穿过县境上长长的隧道，便是雪国。 ← the added 火车 is the English translator's move; it externalizes a viewpoint the original keeps inside the passenger's body.
  > Signed: 穿过县境上长长的隧道便是雪国。 ← Chinese can stay inside; therefore it must.

- **Pronoun ambiguity conserves**: an ambiguous it / cela / es stays ambiguous; when Chinese grammar forces resolution, take the most conservative antecedent — a grammatical necessity, not interpretive license.

- **Tense is a claim** (mechanics in R1/R2; the principle — mark minimally, never fabricate):

  > Darwin, Origin of Species, closing: …endless forms most beautiful and most wonderful have been, and are being, evolved.
  > Collapsed (bad): ……进化出了无数最美丽最奇异的形式。 ← 出了 completes everything; Darwin's double tense IS the claim that evolution is happening now.
  > Signed: ……无数最美且最奇妙的形式已经演化而来，且仍在演化。

- **Historical semantics**: for texts older than ~150 years, the first stop for a puzzling word is a period dictionary (18th-c. *nice* = precise, *awful* = awe-inspiring); and a "rare word" in a scanned old book is more often OCR corruption than a real word — check edit-distance neighbors and a second edition first.

- **Relay seams**: quotations the author read in relay are translated from his relay; discrepancies against the original go into notes, never the body.

- **Lacunae stay lacunae**: damaged source → 「原文缺损」; unadjudicable readings → 正文取最保守读法，注列候选. Never reconstruct.

- **Bibliography and index**: keep searchable source form; translate only connective tissue (See / quoted in / ibid.). Translating titles wholesale fabricates the author's bibliography.

</fine-rules>

<anti-patterns note="self-check every segment">

1. Translating from memory — most common, most fatal; compression turns it into an ambush
2. Translating a relay when the original is obtainable
3. Elegant variation rewriting a fixed term or a fixed grammar marker
4. Explanatory inflation: added 也就是说, added subjects, added connectives the author withheld
5. Added periods; added quote marks; unexplained added punctuation of any kind; comma drift beyond 1 per 1,000 Chinese characters (the Gibbon reflex)
6. Dropped emphasis, footnote markers, final modality; collapsed quotation hierarchy
7. Translating past a compression event without the revival pack
8. "Improving" law-protected strangeness during review
9. Stopping at chapter boundaries / round numbers / post-compression against the gate

</anti-patterns>

<routing note="0-ref by design: this file is the whole skill">
Extraction is not translation: PDF/EPUB/scan/OCR → doc-reader, pdf. Typeset delivery is not translation: Word/PDF/print. This skill owns everything between clean source text and verified Chinese text; per-project scripts (segmenter, concordance, n-gram, punctuation counter) ship with the project, not with the skill.
</routing>

Original byline: KL9 ＆ Claude Fable 5.
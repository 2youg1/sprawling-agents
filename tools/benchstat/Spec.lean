-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import tools.benchstat.spec.Estimators

/-! # benchstat 的规格

`tools/benchstat/` 是不发布、无产品 crate 依赖的基准分析器。它把一轮的
`manifest.json` 与 `results.csv` 变成确定性的 `exports.json` 和权威 Markdown 报告；
同一实现负责生成与验证，读者不需要产品二进制或另一套统计代码。
-/

/-! ## 1 需求分解

| 单元 | 责任 |
|---|---|
| `manifest` | 读取方法版本、分析修订、全局参数、coverage、claims 与 cell 预注册；拒绝缺项和矛盾 |
| `observation` | 按方法学的 28 列表头读取 CSV；验证身份、单位、有限值、任务/request 关联和 span |
| `estimators` | 纯函数：最近秩分位、尾部、OLS、Wilson/Newcombe、Pettitt、Holm、dip 与 KDE |
| `resampling` | SHA-256 派生随机流、Politis–White 块长、moving-block bootstrap、BCa 与 jackknife |
| `analysis` | 按显式 cell key 分组，生成每项统计及其状态、估计器、参数和 seed；不可校准项明确 unavailable |
| `report` | canonical JSON 与 Markdown 受管区段；报告只读导出值 |
| `round` | round 目录读取、摘要校验、原子发布与 verify 字节比较 |
| `main` | `export`、`verify`、`report` 三条命令和退出码；不持分析逻辑 |
-/

/-! ## 2 验收标准

- 同一 manifest、CSV 和分析器版本在两个全新进程产生逐字节相同的 exports 与 README 受管区段。
- `verify` 重算全部导出并逐字节比较 exports；手改受管报告数字、缺文件或输入摘要不符均失败。
- 每个非空定量 cell 导出 n/min/max/P50/P95/P99、相应区间或带原因的空区间、尾部与漂移读数。
- 空 index 只给 iid sensitivity；有顺序时使用不跨 round/cohort 的 moving-block bootstrap。
- `dip_p` 在没有已验证 dependent-null 校准时为 null/unavailable；不得把 iid uniform null 当作确认性校准。
- coverage 的 deferred/failed 行全部进入 Markdown，依赖它们的 claim 标成 deferred。
- `cargo nextest run -p benchstat`、`cargo clippy -p benchstat --all-targets --all-features -- -D warnings`、
  `lake build tools.benchstat.Spec` 通过；历史 `docs/benchmarks/v0.0.11/` 不变。
-/

/-! ## 3 假设与歧义

- manifest 的 `analysis_revision` 是 40 位小写 Git SHA；分析器检查形状和输入摘要，不访问网络或 GitHub。
- `resamples=10000`、`confidence=0.95`、`alpha=0.05`、`power=0.8` 是方法常数；manifest 必须显式写出相同值。
- 预注册 cell plan 以 `key` 关联分析器生成的 cell key；缺 plan 的 cell 可描述，但没有定样或排名资格。
- 自适应 pilot 后合并区间的名义覆盖率不作额外保证；报告必须保留这一限制。
- 本轮没有 HTML 产物。HTML 是后续独立派生物，不是发布或验证条件。
- 没有 calibrated dip p-value 时，KDE 仅作未标记描述：导出 support，中心/切点/share 区间为 not applicable；
  一旦 cell 被标记或预注册要求描述区间，就必须执行完整 shape-vector BCa 或明确失败原因。
-/

/-! ## 4 现状分析

main 上的方法学已经绑定估计量和 schemas，但没有分析器；历史 verifier 只覆盖 v0.0.11 的部分数字。
本包只读取新格式，不改写历史 CSV 或 `verify.py`。历史数据的一次性格式转换与交叉验证只写 `local/`。
-/

/-! ## 5 权威信源

- `docs/benchmarks/methodology.md`：series registry、CSV 字段、估计量、参数和发布政策。
- `docs/performance.md`：独立单位、DiD 和一般解释边界。
- Politis–White (2004) 及 Patton–Politis–White (2009) correction：块长选择。
- Sheather–Jones (1991)、Hartigan AS 217、Pettitt (1979)、Newcombe (1998)：命名算法。
- `spec/Estimators.lean`：最近秩、块长夹取和退化 pilot 下一 cohort 的模型性质。
-/

/-! ## 6 命名统一

Rust 模块与 §1 同名；`CellKey`、`Observation`、`CoverageStatus`、`AnalysisStatus`、`Command` 是唯一拼法。
Lean 的 `nearestRankIndex`、`boundedBlockLength`、`nextPilotSize` 对应 Rust 的同名规则。
-/

/-! ## 7 模块边界

估计器与 resampling 不读文件、时钟、环境变量或网络；输入输出只经调用方给的 slice 和值。
`analysis` 只组合纯值；`report` 只做文本投影；`round` 负责路径与读写，`main` 只负责进程退出。
包不依赖任何产品 crate，不启动线程或子进程，不保留跨调用缓存。
-/

/-! ## 8 接口先行

```text
benchstat export <round-dir> [--write]
benchstat verify <round-dir>
benchstat report <round-dir> [--write]
```

无 `--write` 的 export/report 分别把 exports JSON / Markdown 受管区段写到 stdout，不改文件。
带 `--write` 的 export 原子替换 `exports.json` 与 README 受管区段；report 只重写 README 受管区段。
verify 不写任何文件。退出码：0 成功；1 输入/分析拒绝或 verify 不一致；2 用法、I/O 或内部数值失败。
错误文本为 `CODE action: subject; recovery`，稳定码为 `E_INVALID_INPUT`、`E_VERIFY_MISMATCH`、
`E_USAGE`、`E_IO`、`E_NUMERIC`。

manifest 顶层必填：`schema_version`、`analysis_revision`、`method_version`、`estimator_version`、
`seed_string`、`resamples`、`confidence`、`alpha`、`power`、`bootstrap_unit`、`input_hashes`、
`coverage`、`claims`、`cell_plans`。coverage 使用 `series/status/reason/reopen_when/measured_by/reading`；
cell plan 使用 `key/headline/target_half_width/observations_per_cohort/resource_ceiling/independent_unpaired`，
另可含完整单元映射 `baseline_key`、`absolute_target`、`s0`、`h_abs`，以及独立非配对结局定样所需的
`assumed_first`、`assumed_second`、`family_size`；这些比例只适用于该设计，重复或配对尝试不得使用。

exports 是排序键 JSON，整数保持整数，浮点使用 serde_json 的最短 round-trip 十进制，LF 结尾，无时间戳。
固定六位小数不再作为机器导出格式，因为它会把小斜率写成零。README 的受管标记恰为
`<!-- benchstat:begin -->` 与 `<!-- benchstat:end -->`，各出现一次。
-/

/-! ## 9 工作流程

export：读并验证 → 计算 → canonical exports → 从 exports 渲染 Markdown → 写出或 stdout。
report：读并验证 exports 的形状 → 只从 exports 渲染 Markdown，不重开原始观测。
verify：执行与 export 相同的重算，再比较 exports 字节和 README 受管区段字节。
-/

/-! ## 10 实现逻辑

用量不完整只排除 token 总量；终止证据独立保留。报告标签包含完整 cell 身份。
只有 status=bca 的区间参与确认性定样和 P99 发布。未支持的 KDE 峰保留位置和支持率，
无保留峰时摘要 unavailable，不生成切点或份额。内存定样用 sessions 为空的 pooled key
关联 plan，导出目标、实际半宽和资源上限结果。Markdown 定样列表的代码片段保留 ID 中的 `|`，
逐项显示 status、precision_met、target_half_width 和 achieved_half_width，不倾倒完整 JSON。自由文本声明不能自动解析证伪逻辑；
导出所需观测、方向、阈值和限制，无法执行的声明 unavailable，不能仅凭 coverage 判 evaluated。
结局比较使用候选 plan 的 baseline_key；所有 case 必须一致映射至同一基线汇总，且两组均合格。

随机流是 SHA-256 对 length-prefixed UTF-8 `(seed_string, round, cell key, statistic)` 的计数器输出；
每次 draw 取摘要的下一 8 字节并做拒绝采样，避免模偏差。所有 map 为 BTreeMap，CSV 行序不影响 cell 身份。
moving-block 只在同一 round/cohort 内取块，jackknife 删除同一独立单位。
Politis–White 使用 flat-top lag-window、`M=2*m` 与 2009 年修正；退化相关图返回一块并导出诊断。
KDE 使用 4097 点、Sheather–Jones STE、固定原始网格和标量 FFT 卷积，避免平台 SIMD 造成字节漂移。
-/

/-! ## 11 边界枚举

空 CSV、重复表头/observation id、CRLF、非法 UTF-8、NaN/∞、未知 unit/series、孤儿 request、
反向/越界 span、空 cell、常量 cell、少于两个 jackknife 单位、Pettitt 少于四个有序点、
OLS 少于四 cohort 或三 session level、BCa 分母为零、KDE 无正带宽均必须有显式失败或 unavailable 状态。
输入最多 1,000,000 行；不得由输入声明的容量直接预分配。
-/

/-! ## 12 错误处理

历史交叉验证示例遇到缺失或非对象 cells 时返回 BenchError，使用 E_INVALID_INPUT、
读取动作、exports 文件路径与重新导出的恢复步骤。
输入和分析拒绝保留原文件；verify 不匹配只报告首个不同文件与字段，不回写修正。
写盘经同目录 `.part` 文件、flush、原子替换；失败保留既有发布文件并报告临时路径。
数值不稳定不转成零或默认区间；可用 percentile sensitivity 必须与 BCa 分开标注。
-/

/-! ## 13 依赖选型

serde/serde_json：边界解码与 canonical JSON；sha2：输入摘要和确定性随机流；csv：带引号 CSV；
statrs：正态和二项分布；rustfft（关闭默认 SIMD features）：KDE 卷积；thiserror：错误；tempfile 仅测试。
版本只住 workspace manifests。无产品 crate、网络、随机 OS entropy或并行 runtime。
-/

/-! ## 14 硬编码声明

10000、0.95、0.05、0.8、4097 点、95% mode support、1.25 split-half policy、P99 的 200 样本门槛
由方法学绑定，在 Rust 各只有一处定义；Lean 模型不复制数值表。仅在 maxima 识别时把小于等于峰值 `1e-12` 的密度归零，
这不是统计模态阈值；antimodes 与导出 density 使用未截断的非负 KDE 密度。保留谷底差异，避免归零平台
把切点推向左端；直接公式对照与双峰回归测试同时约束密度保留、谷底切点和假 maxima。
-/

/-! ## 15 影响面

只新增仓库工具和方法学要求的未来轮次 artifacts；不改变产品依赖图、线上协议、客户端或历史轮次。
方法学的导出精度、HTML 和工具位置描述必须与本包同步。
-/

/-! ## 16 测试与约束

`spec/Estimators.lean` 证明：最近秩索引对非空样本总在合法范围；块长总在 1..n；
退化 pilot 的下一目标在 m>0 时严格增加。Rust 侧用闭式答案、公开统计向量、反序 CSV、篡改 exports/README、
常量样本、相关序列、结局分母和端到端全新进程夹具检查对应行为。证明不代替 Rust conformance。
-/

/-! ## 17 文档关系

`docs/benchmarks/methodology.md` 是方法绑定；本规格拥有命令和实现边界；`architecture.toml` 登记模块；
`lakefile.toml` 的两个 `tools.benchstat` glob 收进本规格；`docs/third-party.md` 记录新增 crate。
修改任一命令、字段、估计量或发布政策时，四处按各自权威范围同步审查。
-/

-- D1 独立工具包而非并入 xtask：完整统计核心的泛用性、审计重要性和体量同时满足拆分准则；
-- 它仍不发布且不进入产品图。被否决的方案是 xtask 子命令，因为 xtask 的产品依赖会削弱分析器隔离。
-- D2 本轮只生成 Markdown 与 JSON：HTML 可由 Agent  later 从报告派生，省掉第二视图和维护面；
-- 被否决的是继续让 HTML 参与逐字节发布验证。

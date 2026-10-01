-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# gateway::cost

规定 `cost`（`crates/gateway/src/cost.rs`）：一次调用的结算：权威计费额优先，价目推算全程 checked 整数。本文件是 `crates/gateway/Spec.lean` 的一个分部；下面每一节保留它在 gateway 规格里的标签 §8-n，别处引作 `crates/gateway/Spec.lean §8-n`。

下面的模型证明结算的两条性质：权威计费额在场恒胜；价目推算答出的数就是精确的和、装得进 `u64`，装不下就没有答案。
-/

/-!
### 8-8 gateway::cost（形状 1 判定函数）

```rust
pub struct CallCost { pub billed: UsdMicros, pub source: CostSource, pub usage: ModelUsage }
pub enum CostSource { Authoritative, PriceSheet }
pub fn settle(usage: &ModelUsage, authoritative: Option<UsdMicros>, entry: &ModelEntry) -> Result<CallCost, AxError>;
```

- 权威计费额在场恒胜（`CostSource::Authoritative`）；缺席则按价目推算：`input×input_price/1M + output×output_price/1M + cache 两项`，全程 checked 整数（溢出→E_INVALID_ARGS 报「结算溢出」）。model_returned 载荷含 `billed_usd_micros`＋usage 四整数，A20 对账消费之。
- **四项价格皆零的价目行不结算**：`Endpoint::returned` 只在 `ModelEntry::states_a_price()`（四项价格至少一项非零）时调 `settle`，否则 `billed_usd_micros` 缺席。选型点在目录不认识模型时把四项价格填零，本地模型的行也是零；把它们结算成 `0` 会让账本说「量过了，花了零」，而实际是没有人报过价——`storage::Attribution` 把缺席记为无报价的调用并数它的 token，成本页据此说「没有报价」。落选的是「结算出 0 再由读者猜 0 是否可信」：同一个 0 在两种城里意思相反，读者没有凭据分辨。
-/

namespace Gateway.Cost

/-- 一个 `u64` 装得下的最大数：结算全程是 checked 的 `u64` 算术。 -/
def U64_MAX : Nat := 2 ^ 64 - 1

/-- 价目的单位：每百万 token 的微美元（`cost::TOKENS_PER_PRICE_UNIT`）。 -/
def TOKENS_PER_PRICE_UNIT : Nat := 1000000

/-- 结算出的数从哪里来（`cost::CostSource`）。 -/
inductive CostSource where
  | Authoritative
  | PriceSheet
  deriving DecidableEq, Repr

/-- 一次调用的用量（`kernel::ModelUsage` 的四个整数）。`input_tokens` 是整个 prompt，缓存读写两部分包含在内。 -/
structure ModelUsage where
  input_tokens : Nat
  output_tokens : Nat
  cache_read_tokens : Nat
  cache_write_tokens : Nat

/-- 一个价目行的四项价格（`market::ModelEntry`），每百万 token 的微美元。 -/
structure Prices where
  input_price : Nat
  output_price : Nat
  cache_read_price : Nat
  cache_write_price : Nat

/-- 一次 checked 的 `u64` 运算：越过 `U64_MAX` 就没有答案，恒不回绕。 -/
def checked (n : Nat) : Option Nat :=
  if n ≤ U64_MAX then some n else none

/-- `cost::share`：token 数乘价格（checked），再按百万取整除。 -/
def share (tokens price : Nat) : Option Nat :=
  (checked (tokens * price)).map (· / TOKENS_PER_PRICE_UNIT)

/-- 四项各自的 token 数与价格，按 `settle` 加总的顺序。按输入价计的是 `input_tokens` 减去两个缓存数（Rust 用 `saturating_sub`，`Nat` 的减法同样不低于零）。 -/
def shares (usage : ModelUsage) (prices : Prices) : List (Nat × Nat) :=
  [(usage.input_tokens - usage.cache_read_tokens - usage.cache_write_tokens, prices.input_price),
    (usage.output_tokens, prices.output_price),
    (usage.cache_read_tokens, prices.cache_read_price),
    (usage.cache_write_tokens, prices.cache_write_price)]

/-- 从 `total` 起逐项加上每一份，每一步都 checked。 -/
def settle_from : Nat → List (Nat × Nat) → Option Nat
  | total, [] => some total
  | total, (tokens, price) :: rest =>
    match share tokens price with
    | none => none
    | some part =>
      match checked (total + part) with
      | none => none
      | some next => settle_from next rest

/-- `cost::settle`：权威计费额在场即用它；否则按价目推算，溢出即拒（`E_INVALID_ARGS`）。 -/
def settle (usage : ModelUsage) (authoritative : Option Nat) (prices : Prices) :
    Option (Nat × CostSource) :=
  match authoritative with
  | some billed => some (billed, .Authoritative)
  | none => (settle_from 0 (shares usage prices)).map (fun total => (total, .PriceSheet))

/-- 一份不回绕的整数份额：token 数乘价格再按百万取整除。 -/
def exact (row : Nat × Nat) : Nat := row.1 * row.2 / TOKENS_PER_PRICE_UNIT

/-- 权威计费额在场恒胜，价目推算不是第二个意见。 -/
theorem the_authoritative_amount_always_wins (usage : ModelUsage) (billed : Nat) (prices : Prices) :
    settle usage (some billed) prices = some (billed, .Authoritative) := rfl

theorem checked_keeps_what_fits (n m : Nat) (kept : checked n = some m) : m = n ∧ n ≤ U64_MAX := by
  unfold checked at kept
  by_cases fits : n ≤ U64_MAX
  · simp [fits] at kept
    exact ⟨kept.symm, fits⟩
  · simp [fits] at kept

theorem share_is_exact (tokens price part : Nat) (found : share tokens price = some part) :
    part = exact (tokens, price) := by
  unfold share at found
  cases hc : checked (tokens * price) with
  | none => simp [hc] at found
  | some product =>
    simp [hc] at found
    have kept := checked_keeps_what_fits _ _ hc
    simp [exact, ← found, kept.1]

/-- **结算出的数就是精确的和，恒不回绕。** 价目推算答出一个数时，它等于各份精确份额之和，且装得进 `u64`；装不下的结算没有答案，而不是一个绕回来的小数。 -/
theorem a_settled_sheet_is_the_exact_sum : ∀ (rows : List (Nat × Nat)) (total settled : Nat),
    total ≤ U64_MAX → settle_from total rows = some settled →
      settled = total + (rows.map exact).foldr (· + ·) 0 ∧ settled ≤ U64_MAX
  | [], total, settled, fits, found => by
    simp [settle_from] at found
    subst found
    simp [fits]
  | (tokens, price) :: rest, total, settled, _, found => by
    simp only [settle_from] at found
    cases hs : share tokens price with
    | none => simp [hs] at found
    | some part =>
      cases hc : checked (total + part) with
      | none => simp [hs, hc] at found
      | some next =>
        simp only [hs, hc] at found
        have kept := checked_keeps_what_fits _ _ hc
        have rest_sum := a_settled_sheet_is_the_exact_sum rest next settled (kept.1 ▸ kept.2) found
        have part_exact := share_is_exact tokens price part hs
        refine ⟨?_, rest_sum.2⟩
        rw [rest_sum.1, kept.1, part_exact]
        simp [List.map, Nat.add_assoc]

/-- `cost` 的测试 `the_price_sheet_computes_integer_shares` 那一行：1 Mtok 输入里 200k 读自缓存，100k 输出，按 $3、$15、$0.30。 -/
example : settle ⟨1000000, 100000, 200000, 0⟩ none ⟨3000000, 15000000, 300000, 3750000⟩
    = some (2400000 + 1500000 + 60000, .PriceSheet) := by
  decide

/-- 一份乘积越过 `u64` 的结算没有答案。 -/
example : settle ⟨2 ^ 64, 0, 0, 0⟩ none ⟨2, 0, 0, 0⟩ = none := by
  decide

end Gateway.Cost

-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

import crates.remote_access.spec.Door

/-!
# 远程门的逐帧授权

规定 `crates/sprawling/src/outside/conduit.rs` 的 `Conduit::judge`（`bin::outside::conduit`；§8-139）：一段远程会话里，设备封好的每一帧在到城之前怎样被判。Rust 代码是「怎样守住」的权威；本模型是「必须守住哪些性质」的权威。谁能做什么（`Authority`、`Verb`、`permits`）的权威是 `crates/remote_access/spec/Door.lean`，这里只引用它；一帧属于哪一类由 `bin::outside::verbs::passage` 穷尽匹配给出，在模型里是帧已经带着的类。

打开封装、读 JSON 都可能失败：封装打不开是会话的错误，连接结束；JSON 读不成是一帧封好的 `E_WIRE_MISMATCH` 拒绝，不到城。问候（`Hello`）换上城的令牌再发，版本与 schema 仍是设备的，所以城照样判这个页面说不说它的 wire。

四条性质：

* **到城的只有放行的帧**——转发出去的若不是问候，就是设备的权限 `permits` 的那一类；
* **只看的设备从不转发一个动手的动词**，只在城所在的机器上做的动词（`localOnly`）谁也不转发；
* **读不懂的帧不到城**；
* **会话已不被门持有时什么也不转发**，连接结束。
-/

namespace Sprawling.Outside.Conduit

open RemoteDoor

/-- 打开封装之后读出的东西：锁屏，一帧读不懂的文字，或一帧按类判过的线协议帧。 -/
inductive Opened where
  | lock
  | unreadable
  | greeting
  | judged (verb : Verb)
  deriving Repr, DecidableEq

/-- 判一帧的结果（Rust：`Step`；`Answer` 分出两种拒绝，`Forward` 分出问候）。 -/
inductive Step where
  /-- 放行的原文发给城的 `/ws`。 -/
  | forward
  /-- 问候换上城的令牌再发。 -/
  | forwardGreeting
  /-- 设备收到一帧封好的拒绝：读不懂（`E_WIRE_MISMATCH`）。 -/
  | refuseUnreadable
  /-- 设备收到一帧封好的拒绝：权限不够（`E_GATE_DENIED`）。 -/
  | refuseVerb (verb : Verb)
  /-- 交回给监听去关门。 -/
  | lock
  deriving Repr, DecidableEq

/-- 会话已不被门持有：连接结束。 -/
structure Ended where
  deriving Repr, DecidableEq

/-- `Conduit::judge`。`authority` 是 `Doorway::authority` 此刻的答案，`none` 即会话已不被门持有。 -/
def judge (authority : Option Authority) : Opened → Except Ended Step
  | .lock => .ok .lock
  | .unreadable => .ok .refuseUnreadable
  | .greeting => .ok .forwardGreeting
  | .judged v =>
    match authority with
    | none => .error {}
    | some a => if permits a v then .ok .forward else .ok (.refuseVerb v)

/-- 到城的只有放行的帧。 -/
theorem forwarded_only_if_permitted (authority : Option Authority) (o : Opened)
    (h : judge authority o = .ok .forward) :
    ∃ a v, authority = some a ∧ o = .judged v ∧ permits a v = true := by
  cases o with
  | lock => simp [judge] at h
  | unreadable => simp [judge] at h
  | greeting => simp [judge] at h
  | judged v =>
    cases authority with
    | none => simp [judge] at h
    | some a =>
      by_cases hp : permits a v = true
      · exact ⟨a, v, rfl, rfl, hp⟩
      · simp [judge, hp] at h

/-! 只看的设备从不转发一个动手的动词。 -/

/-- 只在城所在的机器上做的动词（`localOnly`）谁也不转发。 -/
theorem local_only_is_refused (a : Authority) :
    judge (some a) (.judged .localOnly) = .ok (.refuseVerb .localOnly) := by
  simp [judge, local_only_is_never_remote]

/-- 读不懂的帧不到城。 -/
theorem unreadable_reaches_no_city (authority : Option Authority) :
    judge authority .unreadable = .ok .refuseUnreadable := rfl

/-- 会话已不被门持有时，按类判的帧不转发，连接结束。 -/
theorem ended_session_forwards_nothing (v : Verb) : judge none (.judged v) = .error {} := rfl

/-- 只看的设备仍然可以问。 -/
example : judge (some .watch) (.judged .read) = .ok .forward := rfl

end Sprawling.Outside.Conduit

/-!
## 8-139 远程门进城：门的看守、远程监听与逐帧授权（`bin::outside`，形状：adapter；crates/remote_access/Spec.lean §8-10、§8-11，`crates/kernel/Spec.lean` §8-81，`crates/wire/Spec.lean` §8-65、§8-66）

逐帧授权（`Conduit::judge`）必须守住的性质的权威是本文件上面的模型：到城的只有放行的帧，只看的设备从不转发动手的动词，读不懂的帧与会话已结束时都不到城；谁能做什么的权威是 `crates/remote_access/spec/Door.lean`。本节是门的看守、监听与设备表的接口与做法。

一个人把城留在家里出门，要从手机上看城、答提问、叫停。`remote_access` 判谁能进、持有密码学，但它不认识帧、不开端口；这一节是只有本二进制做得了的那一半。

```rust
// bin::outside::keeper：一把锁后面的一扇门
pub(crate) struct Senses { pub(crate) clock: Clock, pub(crate) entropy: Entropy }   // 在 bin::assembly 造
pub(crate) type Choosing = Box<dyn FnMut() -> Result<Box<dyn Route + Send>, AxError> + Send>;      // §8-151
pub(crate) struct Keeping { pub(crate) devices: PathBuf, pub(crate) ledger: Box<dyn Ledger + Send>,
                            pub(crate) choose: Choosing, pub(crate) senses: Senses, pub(crate) key: CityKey }
// bin::outside::keeper::city_key：vault 里的城钥匙
pub(crate) struct CityKey { /* Arc<Mutex<gateway::Custodian>> 与 SecretRef */ }
impl CityKey {
    pub(crate) fn of(vault: Arc<Mutex<gateway::Custodian>>, epoch: Option<B3Hash>) -> Result<CityKey, AxError>;
    pub(super) fn held(&self, entropy: &Entropy) -> Result<SigningKey, AxError>;     // 取回，第一次写
    pub(super) fn replaced(&self, entropy: &Entropy) -> Result<SigningKey, AxError>; // 新种子盖过旧的
    pub(crate) fn lasting(&self) -> Result<Persistence, AxError>;
}
pub(crate) enum Revoking { Named(String), All }
pub(crate) enum Phase { Open, Closed }
#[derive(Clone)] pub(crate) struct Doorway { /* Arc<Mutex<…>> 与 Senses */ }
impl Doorway {
    pub(crate) fn keep(keeping: Keeping) -> Result<Doorway, AxError>;            // 读设备表，门关着
    pub(crate) fn open(&self, local: SocketAddr, lasting: Lasting) -> Result<Opened, AxError>;
    pub(crate) fn attend(&self, listening: Box<dyn Send>) -> Result<(), AxError>;
    pub(crate) fn close(&self, why: RemoteClosing) -> Result<Phase, AxError>;
    pub(crate) fn tick(&self) -> Result<Phase, AxError>;
    pub(crate) fn invite(&self, name: &str, authority: Authority) -> Result<Inviting, AxError>;
    pub(crate) fn pair_reply(&self, hello: &PairHello) -> Result<(PairReply, CityPairing), AxError>;
    pub(crate) fn claim(&self, pairing: CityPairing, sealed: &[u8]) -> Result<Vec<u8>, AxError>;
    pub(crate) fn session_reply(&self, hello: &Hello) -> Result<(Reply, CityWaiting), AxError>;
    pub(crate) fn admit(&self, waiting: CityWaiting, finish: &Finish) -> Result<(SessionId, Session), AxError>;
    pub(crate) fn authority(&self, session: SessionId) -> Result<Option<Authority>, AxError>;
    pub(crate) fn devices(&self) -> Result<Vec<Device>, AxError>;
    pub(crate) fn revoke(&self, which: &Revoking) -> Result<Vec<Device>, AxError>;
    pub(crate) fn replace_key(&self) -> Result<Vec<Device>, AxError>;            // 门开着时拒
    pub(crate) fn key_lasting(&self) -> Result<Persistence, AxError>;
}
// bin::outside::verbs：一帧的类（`crates/wire/Spec.lean` §19-2 的 class 列）
pub(super) enum Passage { Judged(VerbClass), Greeting(wire::Hello) }
pub(super) fn passage(frame: wire::ClientFrame) -> Passage;
pub(super) fn command_class(command: &wire::WireCommand) -> VerbClass;          // 穷尽匹配
// bin::outside::conduit：一段会话的帧，一帧一判
pub(super) enum Step { Forward(String), Answer(Vec<u8>), Lock }
impl Conduit { pub(super) fn judge(&mut self, sealed: &[u8]) -> Result<Step, AxError>;
               pub(super) fn seal_for_device(&mut self, text: &str) -> Result<Vec<u8>, AxError>; }
// bin::outside::listener：回环上的远程监听
pub(crate) const PAIR_PATH: &str = "/remote/pair";
pub(crate) const SESSION_PATH: &str = "/remote/session";
pub(crate) struct Reaching { pub(crate) runtime: Handle, pub(crate) city: SocketAddr, pub(crate) token: Option<String> }
// bin::outside::devices：设备表
pub(super) fn read(path: &Path) -> Result<Vec<Device>, AxError>;
pub(super) fn write(path: &Path, devices: &[Device]) -> Result<(), AxError>;
// bin::assembly::remote_door：门由什么做成，通路怎么选（§8-151）
pub(super) struct Outdoors { city_root, relay, port, key }   // keep(self) -> Result<Remote, AxError>；key 是 CityKey::of 的答案
```

- **一扇门，一把锁**：`Doorway` 是控制台的线程与远程监听的每一条连接共用的句柄。门的一次判定、它写的那一行账、设备表的落盘在同一把锁下发生，所以账本上的次序就是门里发生的次序。
- **五行经 worker 的 relay 写**（`crates/kernel/Spec.lean` §8-81）：`RunWorker::relay()` 交出与驾驶线程同一种 `kernel::Ledger`，草稿过同一个队列到唯一的写者，城仍只有一个写者。`who` 是 `person`，门到时自己关上那一行是 `city`。一行写不进去时，门的那一步不算发生：开门时通路随即关上，配对时设备不留下。
- **设备表**在 `CityLayout::devices`（保留子树下远程门自己的目录里，文件名是 `kernel::layout::DEVICES_FILE`），每台设备一个 `[[device]]`（`id`、`name`、`authority`、`key`，写法见 crates/remote_access/Spec.lean §8-11）。整份写到旁边的新文件再改名盖过去，崩溃只留下改前或改后的一份。文件不存在是空表；读不成的表以 `E_STORAGE_FATAL` 拒并写出文件路径，不猜谁能进。配对与撤销各写一次；写失败时这次配对不算数。
- **名字在门外判唯一**：`invite` 拒一个已配对设备用过的名字（`E_INVALID_ARGS`），门本身不看名字（crates/remote_access/Spec.lean §11）。
- **远程监听**：`/remote open` 在 `127.0.0.1` 上绑一个系统给的端口，先开门、再起接收的任务，任务的中止把手交给门（`attend`），门一关它就停。监听每秒问一次门到没到时；到了就以 `Expired` 关门，写一行。两条路径与上面的消息见 crates/remote_access/Spec.lean §8-10。门的看守里写账、写盘的那几步放到阻塞线程池上做：relay 等的是唯一的写者，套接字任务陪它等会占住一个反应器线程。
- **逐帧授权**：`Conduit::judge` 打开封装，`Lock` 交回给监听去关门；线协议帧读成 `ClientFrame`，`Hello` 换上城的令牌再发（`crates/wire/Spec.lean` §8-66），其余按 `verbs::passage` 判类、问 `Doorway::authority` 与 `permits`。放行的原文发给城的 `/ws`；拒绝的不到城，设备收到一帧封好的 `Refusal`（`E_GATE_DENIED`）；会话已不被门持有时返回错误，连接结束。中继以线协议客户端的身份连城自己的监听，绑在 `0.0.0.0` 的城从回环连。
- **城的签名密钥由 vault 里的种子派生**（crates/remote_access/Spec.lean D23）：引用是 `secret:remote/city-key.<创世链哈希的小写十六进制>`，由 `CityKey::of` 用 `SecretRef::new` 一处造出；创世 id 是 `Views::epoch`，账本还没有创世行时 `of` 以 `E_CONFIG_INVALID` 拒，门的看守取不起来。`Doorway::keep` 经 `CityKey::held` 取回种子、交给 `SigningKey::from_sealed`；vault 答 `E_CREDENTIAL_MISSING` 时才取 32 字节熵，经 `keys::written` 写进去，再从 vault 读回派生，所以第一次与以后每一次走同一条读路。vault 的其他拒绝原样交出。读不成的种子不让门的看守失败：它被记在看守里，邀请与会话握手都交出那句拒绝，`/remote replace-key` 仍能换一把。种子留多久就是 vault 在这个平台上留多久（`crates/gateway/spec/Credential.lean` §8-4）：Windows 凭据管理器与 macOS 钥匙串跨重启；Linux 的内核 keyutils 到这次开机结束，用加密的 vault 文件时跨重启；退回进程内时到城重启为止。
- **更换城钥匙**：`replace_key` 在门开着时以 `E_BUSY` 拒（开着的会话是旧钥匙握的手）；否则 `CityKey::replaced` 取新熵、写进同一个引用盖过旧种子并读回，看守换上新钥匙，再按 `Revoking::All` 撤销每一台设备，各写一行 `device_revoked`，设备表随之写空。vault 写不进去时什么都不变。
- **通路在开门时选，随门开关**：`open` 先判门是否已开，再调一次 `Keeping.choose` 造出这一次的通路并打开它；开着的通路与它答的 `Opened`、监听的任务一起放在门的 standing 里，关门时先停监听，再关通路，再写账。生产装配里 `choose` 读城的 `[remote]`（§8-151）；`outside::tests` 给一条 `route::scripted::ScriptedRoute`。选不出通路（没配、配错）时 `open` 原样交出那句拒绝：门没开，账上没有一行。
- **门的看守取不起来时城照常服务**：设备表读不成、熵取不到，`/remote` 每一个子动词都打印那一句拒绝，其余控制台与页面不受影响。

**测试**：`outside::tests` 用一台 Rust 写的设备、一条脚本化通路、计数的时钟与计数的熵走门：`the_door_writes_its_five_lines_in_the_order_they_happen`（开、配对、会话开始、撤销、关，五行按此次序入账）、`a_local_only_frame_is_refused_and_reaches_no_city`（`Act` 设备发 `Reveal`，答一帧 `E_GATE_DENIED` 的 `Refusal`，没有一个字节放行）、`a_watching_device_is_refused_a_verb_that_acts_and_may_still_ask`（`Watch` 设备的 `Cancel` 被拒、`Ask` 原文放行）、`the_device_table_survives_a_reopen`（两台设备，含一个中文名，重开后设备表相等）、`a_restarted_city_keeps_its_key`（同一个 vault 上两次 `keep`，邀请里的城指纹相同）、`replacing_the_key_unpairs_every_device`（换钥匙之后指纹变了、设备表空了、每台设备一行 `device_revoked`，门开着时拒绝）。
-/

-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# wire::reception::entry

规定 `reception::entry`（`crates/wire/src/` 下同名的文件）。一个请求在读任何凭据之前进不进得了这座城的门：它说的 Host、Origin 与 `Sec-Fetch-Site`，以及它到的是哪条路。本文件是 `crates/wire/Spec.lean` 的一个分部；下面每一节保留它在 wire 规格里的标签 §8-n，别处引作 `crates/wire/Spec.lean §8-n`，决定引作 `wire D<n>`。
-/

/-!
### 8-94 入口判定：`ListenerOrigins` 与 `decide_entry`

```rust
pub struct ListenerOrigins { … }                         // 由绑定后的地址算一次
impl ListenerOrigins {
    pub fn of(local: SocketAddr) -> Self;
    pub fn url(&self) -> &str;                          // 终端印的、`/web` 开的那一个：http://127.0.0.1:<p>
    pub fn admits_host(&self, host: &str) -> bool;
    pub fn admits_origin(&self, origin: &str, host: &str) -> bool;
    pub fn page_headers(&self) -> PageHeaders;          // §8-94 末节
}
pub struct Presented<'a> { pub host: Option<&'a str>, pub origin: Option<&'a str>, pub sec_fetch_site: Option<&'a str> }
pub enum Arrival { Page, Preflight, Socket, Door(Door), Pairing }
pub enum Caller { Native, Browser }
pub enum Entry { Page, Caller(Caller), Refused(EntryRefusal) }
pub enum EntryRefusal { ForeignHost, Preflight, ForeignOrigin, CrossSite, NativePairing }
pub fn decide_entry(origins: &ListenerOrigins, presented: Presented<'_>, arrival: Arrival) -> Entry;
```

判定的次序就是 `decide_entry` 的次序：

1. Host 不在名单：拒（`ForeignHost`）。挡 DNS rebinding 与 `0.0.0.0` 那条路：`Host: evil.example:8787` 与 `Host: 0.0.0.0:8787` 都不在名单。
2. 送页面的 GET（`/`、`/{*asset}`）：放行，带上本监听器的响应头，不问凭据——人要先拿到页面才有地方配对。
3. `OPTIONS`：拒，不回任何 `Access-Control-*`。不开 CORS，所以没有一次预检该成功。
4. 没有 Origin：原生客户端（`Caller::Native`，tungstenite 与 reqwest 都不发）。Origin 在名单里：浏览器（`Caller::Browser`）。别的值，包括 `null`：拒（`ForeignOrigin`）。WS 在 `on_upgrade` 之前就回 403（RFC 6455 §10.2），所以被拒的会话根本不存在。
5. 有 `Sec-Fetch-Site` 而不是 `same-origin`：拒（`CrossSite`）。同站另一个端口给的是 `same-site`，正好拒掉。没有这个头不管：原生客户端不发，三家浏览器在 WS 握手上发不发也没有定论。
6. `/pair` 与 `/session/*` 只收浏览器（`NativePairing`）：设备钥只在页面里生成，原生客户端有本机钥匙，不需要配对。
7. 之后才是凭据（§8-40）：入口判定只说「谁在敲门」，不说「门开不开」。

**名单从绑定后的地址算一次**，读它的有五处：Host 判定、Origin 判定、响应头、`firstrun::local_url`、居民浏览器守卫。没有第二份拼写。

| 绑定 | Host 名单 | Origin 名单 | `url` |
|---|---|---|---|
| 回环（`127.0.0.1`、`::1`） | `127.0.0.1:<p>`、`localhost:<p>`、`[::1]:<p>` | 每个 Host 前加 `http://` | `http://127.0.0.1:<p>`（`::1` 绑定给 `http://[::1]:<p>`） |
| 一个具体的非回环地址 | `<该地址>:<p>` | 同上 | `http://<该地址>:<p>` |
| 未指定地址（`0.0.0.0`、`::`） | 任何 IP 字面量带端口 `<p>`，未指定地址本身除外，外加回环三名 | Origin 必须恰是 `http://` 加这个请求自己的 Host | `http://127.0.0.1:<p>` |

端口 80 时，Host 与 Origin 也认不带端口的写法，因为浏览器对默认端口不写端口。

D54 每个调用方都要非环境凭据，入口先判 Host 与 Origin。
回环端口不是凭据：跨站页面、同站另一个端口的页面、DNS rebinding、同机另一个 OS 用户、居民的两种浏览器都到得了它（`SECURITY.md`）。所以面总带一把钥匙（§8-41），浏览器靠按源存放、不可导出的设备钥换来的会话令牌，原生客户端靠只给本用户读的钥匙文件（`crates/sprawling/spec/Keying.lean` §8-22）。浏览器不会自动附带这两样里的任何一样，所以 Origin 与 Host 的判定是纵深防御，不是唯一一道。被否：会话 cookie——cookie 不按端口隔离（RFC 6265 §8.5），SameSite 不看端口，Safari 在 `http://localhost` 上不发 `Secure` cookie，另一个端口还能写同名 cookie；只判 Origin——挡得住网页，挡不住同机另一个用户与居民的工具。未指定地址的 Origin 必须等于请求自己的 Host，而不是「任何 IP 字面量」：后者让局域网里另一台机器上的页面以浏览器的身份进门（webpack-dev-server CVE-2025-30360 正是这样）。重开条件：浏览器给本机页面一种按源、按端口隔离且不可被脚本读出的凭据。

#### 响应头

每个监听器生成一次，静态字节，作为参数交给 `bundle_routes`：`Content-Security-Policy`（`default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self' <每个 Host 的 ws://>; img-src 'self' data: blob:; worker-src 'self' blob:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`）、`X-Frame-Options: DENY`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`、`Cross-Origin-Opener-Policy: same-origin`、`Cross-Origin-Resource-Policy: same-origin`；不是页面字节的答复另加 `Cache-Control: no-store`。未指定地址的监听器列不出 Host，`connect-src` 写 `'self' ws:`。远程监听器（`bin::outside`）经隧道的公网名到达，名字在这里不可知，用 `PageHeaders::same_origin()`，`connect-src 'self'`。`'wasm-unsafe-eval'` 给 pdf.js 的 wasm 解码器，`'unsafe-inline'` 给 Svelte 写在元素上的样式；页面不再需要它们时删掉。
-/

namespace Wire.Reception.Entry

/-- 请求到的是哪条路（`Arrival`）。 -/
inductive Arrival where
  | page
  | preflight
  | socket
  | door
  | pairing
  deriving DecidableEq, Repr

/-- Origin 头相对名单的三种情形：没有、在名单里、别的（含 `null`）。 -/
inductive OriginSeen where
  | absent
  | listed
  | foreign
  deriving DecidableEq, Repr

/-- `Sec-Fetch-Site` 的三种情形：没有、`same-origin`、别的。 -/
inductive FetchSite where
  | absent
  | sameOrigin
  | other
  deriving DecidableEq, Repr

inductive Caller where
  | native
  | browser
  deriving DecidableEq, Repr

inductive Refusal where
  | foreignHost
  | preflight
  | foreignOrigin
  | crossSite
  | nativePairing
  deriving DecidableEq, Repr

inductive Entry where
  | page
  | caller (who : Caller)
  | refused (why : Refusal)
  deriving DecidableEq, Repr

/-- 第 4 步：Origin 说出谁在敲门。 -/
def callerOf : OriginSeen → Option Caller
  | .absent => some .native
  | .listed => some .browser
  | .foreign => none

/-- `decide_entry`，次序即 §8-94 的七步（第 7 步在 §8-40）。`hostListed` 是 `ListenerOrigins::admits_host` 的答案。 -/
def decideEntry (hostListed : Bool) (origin : OriginSeen) (site : FetchSite) (arrival : Arrival) : Entry :=
  if !hostListed then .refused .foreignHost
  else if arrival = .page then .page
  else if arrival = .preflight then .refused .preflight
  else match callerOf origin with
    | none => .refused .foreignOrigin
    | some who =>
      if site = .other then .refused .crossSite
      else if arrival = .pairing ∧ who = .native then .refused .nativePairing
      else .caller who

/-- **Host 不在名单的请求从不进门**，连页面也拿不到。 -/
theorem a_foreign_host_never_enters (origin : OriginSeen) (site : FetchSite) (arrival : Arrival) :
    decideEntry false origin site arrival = .refused .foreignHost := by
  simp [decideEntry]

/-- **Origin 不在名单的请求，除了取页面，从不进门。** -/
theorem a_foreign_origin_never_enters (host : Bool) (site : FetchSite) (arrival : Arrival)
    (notPage : arrival ≠ .page) : ∀ who, decideEntry host .foreign site arrival ≠ .caller who := by
  intro who
  cases host <;> cases arrival <;> simp_all [decideEntry, callerOf]

/-- **一次跨站或同站另一端口的取数从不进门**（页面字节除外）。 -/
theorem a_cross_site_fetch_never_enters (host : Bool) (origin : OriginSeen) (arrival : Arrival)
    (notPage : arrival ≠ .page) : ∀ who, decideEntry host origin .other arrival ≠ .caller who := by
  intro who
  cases host <;> cases arrival <;> cases origin <;> simp_all [decideEntry, callerOf]

/-- **预检从不进门。** -/
theorem a_preflight_never_enters (host : Bool) (origin : OriginSeen) (site : FetchSite) :
    ∀ who, decideEntry host origin site .preflight ≠ .caller who := by
  intro who
  cases host <;> simp [decideEntry]

/-- **配对与会话的路只进浏览器。** -/
theorem pairing_admits_only_a_browser (host : Bool) (origin : OriginSeen) (site : FetchSite) (who : Caller)
    (entered : decideEntry host origin site .pairing = .caller who) : who = .browser := by
  cases host <;> cases origin <;> cases site <;> simp_all [decideEntry, callerOf] <;>
    first | exact entered.symm | (cases entered)

/-- 一条可以实现的正常路径：本机页面的 hello 进门，原生客户端的 POST 进门。 -/
example : decideEntry true .listed .sameOrigin .socket = .caller .browser := rfl
example : decideEntry true .absent .absent .door = .caller .native := rfl

end Wire.Reception.Entry

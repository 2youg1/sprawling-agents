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
    pub fn routed(local: SocketAddr, public: &str) -> Self;  // 远程监听：local 的回环三名，外加通路给的 https 地址
    pub fn url(&self) -> &str;                          // 终端印的、`/web` 开的那一个：http://127.0.0.1:<p>
    pub fn admits_host(&self, host: &str) -> bool;
    pub fn admits_origin(&self, origin: &str, host: &str) -> bool;
}
impl PageHeaders { pub fn every_listener() -> Self; }   // §8-94 末节
pub struct Presented<'a> { pub host: Option<&'a str>, pub origin: Option<&'a str>, pub sec_fetch_site: Option<&'a str> }
pub enum Arrival { Page, Preflight, Socket, Door(Door), Pairing }
pub enum Caller { Native, Browser }
pub enum Entry { Page, Caller(Caller), Refused(EntryRefusal) }
pub enum EntryRefusal { ForeignHost, Preflight, ForeignOrigin, CrossSite, NativePairing }
pub fn decide_entry(origins: &ListenerOrigins, presented: Presented<'_>, arrival: Arrival) -> Entry;
// wire::server::guard，在 crate 根再导出：一张路由表的每条路都按 arrival 先过入口判定，任何一条上的 OPTIONS 按预检判
pub fn entered<S>(routes: Router<S>, arrival: Arrival, origins: &ListenerOrigins) -> Router<S>;
```

判定的次序就是 `decide_entry` 的次序：

1. Host 不在名单：拒（`ForeignHost`）。挡 DNS rebinding 与 `0.0.0.0` 那条路：`Host: evil.example:8787` 与 `Host: 0.0.0.0:8787` 都不在名单。
2. 送页面的 GET（`/`、`/{*asset}`）：放行，带上本监听器的响应头，不问凭据——人要先拿到页面才有地方配对。
3. `OPTIONS`：拒，不回任何 `Access-Control-*`。不开 CORS，所以没有一次预检该成功。
4. 没有 Origin：原生客户端（`Caller::Native`，tungstenite 与 reqwest 都不发）。Origin 在名单里：浏览器（`Caller::Browser`）。别的值，包括 `null`：拒（`ForeignOrigin`）。WS 在 `on_upgrade` 之前就回 403（RFC 6455 §10.2），所以被拒的会话根本不存在。
5. 有 `Sec-Fetch-Site` 而不是 `same-origin`：拒（`CrossSite`）。同站另一个端口给的是 `same-site`，正好拒掉。没有这个头不管：原生客户端不发，三家浏览器在 WS 握手上发不发也没有定论。
6. `/pair` 与 `/session/*` 只收浏览器（`NativePairing`）：设备钥只在页面里生成，原生客户端有native key，不需要配对。
7. 之后才是凭据（§8-40）：入口判定只说「谁在敲门」，不说「门开不开」。

**名单从绑定后的地址算一次**，读它的有四处：Host 判定、Origin 判定、`firstrun` 开浏览器与终端印的地址（`ListenerOrigins::url`）、居民浏览器守卫。没有第二份拼写。

| 绑定 | Host 名单 | Origin 名单 | `url` |
|---|---|---|---|
| 回环（`127.0.0.1`、`::1`） | `127.0.0.1:<p>`、`localhost:<p>`、`[::1]:<p>` | 每个 Host 前加 `http://` | `http://127.0.0.1:<p>`（`::1` 绑定给 `http://[::1]:<p>`） |
| 一个具体的非回环地址 | `<该地址>:<p>` | 同上 | `http://<该地址>:<p>` |
| 未指定地址（`0.0.0.0`、`::`） | 任何 IP 字面量带端口 `<p>`，未指定地址本身除外，外加回环三名 | Origin 必须恰是 `http://` 加这个请求自己的 Host | `http://127.0.0.1:<p>` |
| 远程监听（`routed`，回环端口 `<p>`，通路地址 `https://<名>`） | 回环三名，外加 `<名>` | 回环三名前加 `http://`，外加 `https://<名>` | `http://127.0.0.1:<p>` |

端口 80 时，Host 与 Origin 也认不带端口的写法，因为浏览器对默认端口不写端口。
通路地址的 `<名>` 是 `https://` 之后、第一个 `/` 或 `?` 之前的那一段，按 ASCII 不分大小写比较；写成 `:443` 的端口去掉，因为浏览器对 `https` 的默认端口不写端口。

D56 远程监听有自己的名单，由同一个 `decide_entry` 判。它的两条 WebSocket 路（`/remote/pair`、`/remote/session`）按 `Arrival::Socket` 到，页面按 `Arrival::Page` 到。名单里有回环三名，也有通路给的公网名：通路把请求交到回环端口时，可能保留浏览器写的 `Host`（公网名），也可能改写成回环地址，两种都要进得来；这两种之外的 Host 是 DNS rebinding。浏览器在通路上的页面发的 Origin 是 `https://<名>`，别的 Origin 是另一个站点的页面，在升级之前就回 403。两条路按 `Socket` 而不按 `Pairing` 到：远程的握手由钉住的设备钥与终端确认码认证（`crates/remote_access/Spec.lean` §8-6），不靠「只收浏览器」；隔着网络的程序本来就能写任何 Origin，拒绝没有 Origin 的请求只会拒掉城所在那台电脑上的原生客户端，挡不住谁。这一层是纵深防御，挡的是浏览器里另一个站点的页面。被否：远程监听不判 Host 与 Origin（另一个站点的页面能打开到设备握手的连接，只剩握手这一道）；只列公网名（会把改写 Host 的通路与从回环直连的客户端都拒掉）。重开条件：通路给出它转发时写的 Host，或者远程的握手改成依赖浏览器的身份。

D54 每个调用方都要非环境凭据，入口先判 Host 与 Origin。
回环端口不是凭据：跨站页面、同站另一个端口的页面、DNS rebinding、同机另一个 OS 用户、居民的两种浏览器都到得了它（`SECURITY.md`）。所以面总带一把钥匙（§8-41），浏览器靠按源存放、不可导出的设备钥换来的会话令牌，原生客户端靠只给本用户读的钥匙文件（`crates/sprawling/spec/Keying.lean` §8-22）。浏览器不会自动附带这两样里的任何一样，所以 Origin 与 Host 的判定是纵深防御，不是唯一一道。被否：会话 cookie——cookie 不按端口隔离（RFC 6265 §8.5），SameSite 不看端口，Safari 在 `http://localhost` 上不发 `Secure` cookie，另一个端口还能写同名 cookie；只判 Origin——挡得住网页，挡不住同机另一个用户与居民的工具。未指定地址的 Origin 必须等于请求自己的 Host，而不是「任何 IP 字面量」：后者让局域网里另一台机器上的页面以浏览器的身份进门（webpack-dev-server CVE-2025-30360 正是这样）。重开条件：浏览器给这台电脑上的页面一种按源、按端口隔离且不可被脚本读出的凭据。

#### 响应头

一套头，每个监听器把它转成 HTTP 头一次、作为参数交给 `bundle_routes`：`Content-Security-Policy`（`default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data: blob:; worker-src 'self' blob:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`）、`X-Frame-Options: DENY`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`、`Cross-Origin-Opener-Policy: same-origin`、`Cross-Origin-Resource-Policy: same-origin`；不是页面字节的答复另加 `Cache-Control: no-store`。

- **`connect-src 'self'`，不逐个列 `ws://<Host>`**：CSP Level 3 让 `'self'` 在 `http:` 页上认同一主机与端口的 `ws:`、在 `https:` 页上认 `wss:`，配对要用的 WebCrypto Ed25519 在 Chromium 137、Gecko 129、WebKit 17 才有，这几版都按这条规则匹配。于是城自己的端口与远程监听器（经隧道的公网名到达，名字在这里不可知）送的是同一套头，D20「同样的字节、同样的头」仍成立。被否：按监听器列出每个 Host 的 `ws://`——远程监听器列不出，未指定地址的监听器也列不出，两张表只会让同一个页面在两扇门后受两种策略。
- `'wasm-unsafe-eval'` 给 pdf.js 的 wasm 解码器，`'unsafe-inline'` 给 Svelte 写在元素上的样式；页面不再需要它们时删掉。
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

/-- 一条可以实现的正常路径：这台电脑上的页面的 hello 进门，原生客户端的 POST 进门。 -/
example : decideEntry true .listed .sameOrigin .socket = .caller .browser := rfl
example : decideEntry true .absent .absent .door = .caller .native := rfl

end Wire.Reception.Entry

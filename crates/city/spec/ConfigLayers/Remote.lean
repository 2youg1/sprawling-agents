-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::config_layers::remote

规定 `config_layers::remote`（`crates/city/src/` 下同名的文件）。城那一层选远程门的通路。本文件是 `crates/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `crates/city/Spec.lean §8-n`，决定引作 `city D<n>`。
-/

/-!
### 8-39 `[remote]`：城那一层选远程门的通路（`config_layers::remote`，形状 1 判定）

**接口**：

```rust
// city::config_layers::remote；对外 city::{remote_route, RemoteRoute, HostPermanence}
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteRoute {
    Cloudflare { tunnel: String, url: String, command: Option<String> },
    Command { command: String, args: Vec<String>, permanence: HostPermanence },
}
pub enum HostPermanence { Fixed, PerStart }       // 拼法 "fixed" | "per_start"
pub fn remote_route(city_root: &Path) -> Result<RemoteRoute, AxError>;
impl ConfigLayer { pub fn remote(&self) -> Option<&RemoteRoute>; }
```

```toml
[remote]
route = "cloudflare"
tunnel = "my-city"                  # `cloudflared tunnel create` 时起的名字
url = "https://city.example.org"    # 隧道的 DNS 路由指向的主机
# command = "C:/tools/cloudflared.exe"   # 不写时从 PATH 找 cloudflared

[remote]
route = "command"
command = "sh"
args = ["/path/to/tailscale-route.sh"]
permanence = "fixed"                # "fixed" | "per_start"
```

- **只有城那一层能写**：远程门开在整座城上，一栋楼或一个房间写下通路，就是一份下层的文件替全城决定外面从哪里进来。楼层、房间层写 `[remote]`，`ladder::stated` 读到它即拒（`E_CONFIG_INVALID`），恢复语指向城根的 `.sprawling/CONFIG.toml`。这与 `[skills] shelves`（§8-8）是同一条判定：`refuse::CityOnly` 有 `Shelves`、`Remote` 两臂，各带键名与理由，`ConfigLayer::city_only` 答一层写下的第一张这样的表，拒法只有 `refuse::below_city` 一处。
- **`route` 选臂**：`route` 是闭集（`cloudflare`｜`command`），每一臂只收自己的键。别的臂的键、不认的键、缺 `route`、缺本臂必需的键（`cloudflare` 的 `tunnel` 与 `url`，`command` 的 `command` 与 `permanence`）都由 serde 拒，主体点名那个键，拒法是本模块既有的 `refuse::unreadable`。`command`＋`args` 与 `[[mcp]]` 的同名两键同形，人认一次；`command` 为空串即拒。
- **值照写下的读进来**：隧道名合不合 `cloudflared` 的写法、`url` 是不是 `https://`、程序在不在，由 `remote_access` 的类型判（`TunnelName::parse`、`PublicUrl::parse`，`crates/remote_access/Spec.lean` §8-7 到 §8-9），判在装配层每次 `/remote open` 把这张表造成通路的那一刻（sprawling-SPEC 8-151）。本 crate 只见 `kernel`，理由与 `[resident] harness` 相同（D16）。
- **`permanence` 必写，没有缺省**：命令证明不了自己的主机名跨重启不变（`crates/remote_access/Spec.lean` §8-9）。缺省成 `fixed`，一条每次换主机名的通路就会被当成不换，设备的密钥在下次重启后读不到，而控制台没有提醒过人。`cloudflare` 一臂没有这个键：命名隧道的主机名是人用 DNS 路由钉住的，答 `Fixed`。
- **`remote_route` 只读城那一层，不爬梯子**：与 `city_shelves` 同一种读法（`ladder::stated(city_config, Layer::City)`）。文件不在、或文件里没有 `[remote]`，答 `E_CONFIG_INVALID`：主体是那份文件与「no `[remote]` table chooses a route」，恢复语写出两臂各自必需的键并指向 `docs/operating.md`。缺表对唯一的调用方 `/remote open` 就是拒绝，所以不答 `Option`。

**测试**（`config_layers::remote::tests`）：两臂各读回整个值；`cloudflare` 缺 `url` 即拒，主体点名 `url`；楼层写 `[remote]` 即拒，恢复语指向城那一层；城那一层没有 `[remote]` 时，`remote_route` 的拒绝点名这张表与两臂必需的键。
-/

/-! D16 `[remote]` 只在城那一层，值照写下的读，判在开门时

**决定**：`[remote]` 是城那一层独有的表，`route` 选臂；隧道名、地址与程序以字符串交给装配层，在每次 `/remote open` 时由 `remote_access` 的类型判（§8-39）。

**理由**：远程门开在整座城上，选通路的只能是管这座城的那份文件，下层写它与下层挂外部书架是同一种越权，所以两张表共用一条判定与一个拒法，而不是各写一份。本 crate 在拓扑上只依赖 `kernel`；隧道名与 `https://` 地址的规则已在 `remote_access` 有唯一的家，在这里再写一遍，就是同一条规则两个权威，两份迟早说法不一。

**被否**：①`[remote]` 上三层梯子：一栋楼能替全城开一条外面进来的路；②在本 crate 复写 `TunnelName`、`PublicUrl` 的判定，让写错的值在解析点就被拒：两个权威；③`permanence` 缺省为 `fixed`：见 §8-39，错的缺省比没有缺省更难诊断。

**重开参数**：本 crate 被允许依赖 `remote_access`（拓扑改动）时，解析点直接造出类型化的值，写错的地址在读文件时就被拒。
-/

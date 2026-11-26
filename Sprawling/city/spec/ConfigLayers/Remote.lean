-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
-- Copyright (c) 2026 2youg1 and the sprawling contributors

/-!
# city::config_layers::remote

规定 `config_layers::remote`（`Sprawling/city/src/` 下同名的文件）。城那一层选远程门的通路。本文件是 `Sprawling/city/Spec.lean` 的一个分部；下面每一节保留它在 city 规格里的标签 §8-n，别处引作 `Sprawling/city/Spec.lean §8-n`，决定引作 `city D<n>`。

这一分部只有文字：它是说明文档，不是形式规格，这里没有一句是被证明的；它写下的接口形状与取舍由 Rust 的类型与 `city::config_layers::remote` 旁的测试守住。
-/

/-!
### 8-39 `[remote]`：城那一层选远程门的通路（`config_layers::remote`，形状 1 判定）

**接口**：

```rust
#[serde(tag = "route", rename_all = "snake_case", deny_unknown_fields)]
pub enum RemoteRoute {
    H3 { url: String, listen: SocketAddr, certificate: String, private_key: String },
}
pub fn remote_route(city_root: &Path) -> Result<RemoteRoute, AxError>;
impl ConfigLayer { pub fn remote(&self) -> Option<&RemoteRoute>; }
```

```toml
[remote]
route = "h3"
url = "https://city.example.org"
listen = "0.0.0.0:443"
certificate = "/absolute/path/to/cert.pem"
private_key = "/absolute/path/to/key.pem"
```

- 只有城级文件能写 `[remote]`。楼层、房间层的表由 `refuse::too_near` 拒绝。
- `remote_route` 使用 `ladder::stated(city_config, Layer::City)` 读取城级文件。表缺失、路由种类未知、存在多余键或缺必需键都答 `E_CONFIG_INVALID`。过去的 h3 表删去 `command`，保留其余四个键。
- 装配层每次开门验证 HTTPS 源、PEM 路径和 UDP 监听设置。配置错误时门仍关闭。测试用 `cargo test -p sprawling-city config_layers::remote` 检查读取、迁移拒绝和层级限制。
-/

/-! D16 公网入口的配置归城级文件，运行参数交给装配层验证

远程门服务整座城，所以只有城级文件能配置它。TLS 和绑定错误依赖实际证书与端口，统一在开门时验证。之前支持启动外部通路程序，WebTransport 直接由城监听 UDP 后只需要保存源、监听地址、证书和密钥。用一条通用命令交出任意路由的设计既不能检查公网协议，也不能用同一生命周期关闭所有连接。
-/

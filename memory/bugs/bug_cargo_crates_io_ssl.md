---
name: 国内网络下 cargo 拉 crates.io 报 SSL 错
description: 在国内网络环境下 `cargo build` / `tauri dev` 拉 crates.io 报 "SSL connect error" 或 "download of config.json failed"
type: feedback
---

# 国内网络下 cargo 拉 crates.io 报 SSL 错

**规则**：国内环境必须把 cargo 配成 rsproxy sparse 镜像，否则 `tauri dev` 第一次构建必失败。

---

**Why**：

直接拉 `https://crates.io` 的 git 索引或 https 接口经常超时/SSL EOF。sparse 协议走 `sparse+https://rsproxy.cn/index/`，纯 HTTPS 拉小文件，命中国内 CDN。

**触发条件**：
- 网络是国内 ISP
- `~/.cargo/config.toml` 没有配 `replace-with`
- 第一次 `tauri dev`（要拉一大堆依赖）

**示例报错**：
```
error: failed to load source for dependency `serde`
Caused by: download of config.json failed
Caused by: curl failed
Caused by: [35] SSL connect error (TLS connect error: ...unexpected eof while reading)
```

或 sparse 配错时：
```
fatal: 仓库 'https://rsproxy.cn/crates.io-index/' 未找到
```
（错把 git 协议用成了 https git URL，必须是 `sparse+https://...`）

---

**How to apply**：

- **新机器** 第一步就写 `~/.cargo/config.toml`：
  ```toml
  [source.crates-io]
  replace-with = "rsproxy-sparse"

  [source.rsproxy-sparse]
  registry = "sparse+https://rsproxy.cn/index/"

  [net]
  git-fetch-with-cli = true
  ```
- **排查**：先 `curl -I https://rsproxy.cn/index/config.json` 看通不通
- **备选镜像**：tuna、ustc，配置方法类似，把 URL 换掉

---

**环境依赖**：
- 仅限国内网络环境
- cargo 1.68+（sparse 协议要求）

---

**首次发现**：2026-06-06，`tauri dev` 首次拉依赖
**相关提交**：cab4b27（CLAUDE.md 提到了镜像，但未在 commit 之外的动作中自动化）

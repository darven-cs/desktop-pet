---
name: tag 已发布后修 bug 用 --force 移动 tag
description: GitHub release 已发布后发现 bug 时，强制移动 tag 走 --force 让 CI 重跑并 update 旧 release
type: feedback
---

# Tag 已发布后修 bug：移动 tag 走 --force

**场景**：tag `v0.x` 已 push 触发 CI，CI 完成后 release 已发到 GitHub Releases；这时候发现 bug fix 还没进 release。

**规则**：直接 `git tag -f v0.x <fix-commit-sha> && git push --force origin v0.x`，让 CI 用新 SHA 重跑，`softprops/action-gh-release` 看到同名 tag 会 update 旧 release（不创建新的）。

---

**Why**：

- Tauri 项目的 release 是 tag push 触发的（`.github/workflows/build.yml` 里 `tags: ["v*"]`）
- CI 跑完到 release 发布有 5-10 分钟（macOS + Windows build 慢），这段时间任何 master 推送都是 branch build，不进 release
- 走 --force 移动 tag → CI 用新 commit 重跑 → `create-release` job 看到同名 tag 直接 update（不是新建）
- 用户从 GitHub Releases 拿到的就是带 fix 的版本

**对比另两个选项**：

| 选项 | 缺点 |
|---|---|
| bump 到 v0.x.1 再 tag | 多一个 release 用户搞不清；坏版本留在线上 |
| 不动，master 后续 release 修 | 已下载 v0.x 的用户拿到坏版本 |

---

**How to apply**：

移动 tag 之前先确认：
1. CI 状态：旧 tag 触发的 build 已 completed（success），旧 release 已发布
2. fix commit 已经在 master 上推了
3. 然后：
   ```bash
   git tag -d v0.x
   git tag v0.x <fix-commit-sha>
   git push --force origin v0.x
   ```
4. 监控：CI 用新 SHA 触发新 run，run 完 `create-release` job 会 update 旧 release（同名 tag），assets 时间戳变化

**验证 release 已更新**：
```bash
gh release view v0.x --json assets --jq '.assets[].updatedAt'
```

**和"不要 force-push 改分支"不同**：force-push tag 是 GitHub 明确支持的 release 修正操作，不会触发 review。force-push 改分支历史才危险。

**首次实践**：2026-06-07 v0.2.0（8 个新动画 + spec 05 + 退出 bug + Mermaid bug 一起进了 release）

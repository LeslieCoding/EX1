# EX1

EX1 是 Windows x64 原生 Xray 代理客户端，使用 Tauri + Rust + 原生 HTML/CSS/TypeScript。支持订阅管理、VLESS 节点选择、自动优选、后台刷新和 Windows 系统代理。

## 构建

```bash
pnpm install
pnpm web:build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

应用配置位于 `%APPDATA%\EX1`，代理端口默认 `127.0.0.1:10808`（界面可改，系统代理与核心共用同一端口）；订阅默认每 15 分钟自动更新并在更新后重新选优，间隔可在界面切换 5/15/30/60 分钟。详细规格与验收记录见 `AI_SPEC/`。

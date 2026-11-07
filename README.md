# EX1

EX1 是 Windows x64 原生 Xray 代理客户端，使用 Tauri + Rust + 原生 HTML/CSS/TypeScript。支持订阅管理、VLESS 节点选择、自动优选、后台刷新和 Windows 系统代理。

## 构建

```bash
pnpm install
pnpm web:build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

应用配置位于 `%APPDATA%\EX1`，代理端口默认 `127.0.0.1:10808`（界面可改，系统代理与核心共用同一端口）；订阅默认每 5 分钟自动更新并在更新后重新选优，间隔可在界面切换 3/5/10/15/30/60 分钟。详细规格与验收记录见 `AI_SPEC/`。

## 自动更新

顶栏「检查更新」按钮可手动检查并一键更新；侧栏「自动更新 EX1」开关控制启动后与每 6 小时自动检查（发现新版本自动下载并静默安装重启，同一版本只自动装一次）。

更新源为公开仓库 `https://github.com/LeslieCoding/EX1-updates`（主仓库私有，APP 匿名读不到 release，故单独公开一份安装包与更新说明）。测试时可设置环境变量 `EX1_UPDATE_FEED` 指向自定义 feed 地址。

# EX1

EX1 是 Windows x64 和 Linux x64 原生代理客户端，使用 Tauri + Rust + 原生 HTML/CSS/TypeScript。支持订阅管理、VLESS / VMess / Hysteria2 / AnyTLS 节点选择、自动优选、后台刷新和系统代理。Windows 使用 Xray 与 mihomo，Linux 使用 mihomo 混合端口。

## 构建

```bash
pnpm install
pnpm web:build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
```

Linux 正式构建使用 `bash scripts/build-linux.sh`，产物为 `src-tauri/target/release/bundle/deb/EX1_<版本>_amd64.deb`。

Windows 配置位于 `%APPDATA%\EX1`，Linux 配置位于 `$XDG_CONFIG_HOME/EX1` 或 `~/.config/EX1`。代理端口默认 `127.0.0.1:10808`（界面可改，系统代理与核心共用同一端口）；Windows 订阅默认每 15 分钟自动更新，普通后台刷新在当前节点有效且连接配置不变时不全量测速，失效或配置变化时立即恢复，另有每 15 分钟独立全量选优；更新间隔可在界面切换 15/30/60/120/360 分钟。详细规格与验收记录见 `AI_SPEC/`。

Linux 使用 `systemd --user` 的 `ex1.service` 负责登录自启动和异常退出恢复，应用内每 10 秒检查代理核心并自动拉起。独立的联网看门狗每 20 秒经当前代理请求真实 HTTPS，连续 2 次失败立即刷新订阅并重新选优（5 分钟冷却），健康时每 2 分钟独立选优；这些任务与订阅周期刷新分开。订阅内容没有影响当前节点时不重启核心。

系统代理只由应用内的手动开关控制，应用启动和退出不会强制切换 GNOME 系统代理。开关和地址显示 GNOME 的真实状态，前台每 2 秒同步，不用持久化缓存冒充实际状态。Linux TUN 需要额外管理员网络权限，当前便携部署不启用。

## 自动更新

Windows 顶栏「检查更新」按钮可手动检查并一键更新；侧栏「自动更新 EX1」开关控制启动后与每 6 小时自动检查（发现新版本自动下载并静默安装重启，同一版本只自动装一次）。Linux 便携版目前只检查版本，不自动安装应用更新；订阅自动更新不受影响。

更新源为公开仓库 `https://github.com/LeslieCoding/EX1` 的最新 Release，安装包与更新说明均随主仓库发布。测试时可设置环境变量 `EX1_UPDATE_FEED` 指向自定义 feed 地址。

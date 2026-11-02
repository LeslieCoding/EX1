# EX1 开发与验收记录

## 架构
- Tauri 原生窗口 + Rust 后端 + 原生 HTML/CSS/TypeScript 前端。
- Xray 26.10.21（官方安全更新版），打包于 `src-tauri/sidecar/xray.exe`，运行时不依赖 v2rayN 路径。
- 配置使用新版 `socks` 入站和 `websocket` 传输字段，不依赖 GeoIP/GeoSite 数据文件。
- 订阅 YAML 与 VLESS 链接解析、并行全部更新、当前订阅后台更新。
- 真实 Xray 逐节点测速；自动选择开启时前后端均禁止手选。

## 关键修正
- 移除生产环境 devUrl，确保 exe 加载内嵌前端。
- 初始化 Tokio runtime，避免窗口启动时缺少 reactor。
- 系统代理保存原 ProxyEnable/ProxyServer/ProxyOverride/AutoConfigURL/AutoDetect；只恢复 EX1 自己写入的状态，并发送 WinINet 刷新通知。
- 所有 Tauri 写命令返回统一 PublicState，后台任务复用独立实现函数。

## 验证
- `cargo test`：3/3 通过（YAML、VLESS URI、地区过滤）。
- `pnpm web:build`：TypeScript 和 Vite 构建通过。
- 真实 EdgeTunnel 订阅通过 UI 添加，成功导入节点；后台更新正常。
- 自动选择无境外节点时显示可理解的提示；关闭自动选择后可手动选择 CF 节点。
- 真实 UI 启动代理后，当前节点正确、Windows 系统代理生效，YouTube 返回 HTTP 200。
- 停止代理后恢复原系统代理地址 127.0.0.1:7897。
- 真实窗口截图确认暖白 + #E60012 主题，启动及运行过程没有终端窗口。
- 打包改用本机 pnpm 避免 Corepack 重复下载；安装模式为当前用户，`/S` 静默安装返回 0。
- 安装版位于 `%LOCALAPPDATA%\EX1\ex1.exe`，随包 Xray 位于同目录 `sidecar\xray.exe`，实测 YouTube HTTP 200。
- GitHub 私有仓库、v0.1.0 tag、Release 与 Windows 安装包均已发布并核验。

## 内核来源
- Xray 26.10.21 官方 Release。
- ZIP SHA256：`fc58c39ee2f2717b4c54d28d374362389e5319b20ae44aea76ea45a5243ec827`。
- `xray.exe -version` 实测为 Xray 26.10.21。

## v0.1.1 品牌图标重制
- 废弃旧版红底“X1”文字图标和左上角 CSS 字母拼接，改为统一的独立路由节点与防护外框几何标志。
- 保留明亮标准红 `#E60012`，增加同色系明暗层次；新增可复用矢量源 `src/ex1-mark.svg`。
- Windows ICO 包含 16、24、32、48、64、128、256 像素；应用 PNG 使用高分辨率抗锯齿输出。
- 左上角改用同一 SVG 标志，未改动 Xray、订阅、节点选择和系统代理逻辑。
- `cargo test` 3/3、`pnpm web:build`、`pnpm tauri build` 均通过。
- v0.1.1 NSIS `/S` 静默安装返回 0；安装版 ProductVersion 为 0.1.1。
- 真实桌面截图确认桌面快捷方式、窗口标题栏和界面左上角均显示新图标，旧快捷方式缓存已刷新。
- 安装版启动无终端窗口，经 `127.0.0.1:10808` 请求 YouTube 返回 HTTP 200。

## v0.1.2 翡翠绿主题
- 全套主题由标准红改为低饱和翡翠绿 `#189162`，使用浅绿 hover/selected 状态与同色系焦点、阴影，不使用荧光绿。
- 错误、表单错误和删除确认保留独立红色语义，未把危险操作混入主题色。
- SVG、PNG、Windows ICO、桌面、开始菜单、窗口标题栏、界面左上角及任务栏统一换色。
- 版本号同步升至 0.1.2；未改动 Xray、订阅、节点选择和系统代理逻辑。
- `cargo test` 3/3、`pnpm web:build`、`pnpm tauri build` 均通过。
- v0.1.2 NSIS `/S` 静默安装返回 0；安装版 ProductVersion 为 0.1.2。
- 真实窗口和任务栏 PrintWindow 截图均确认绿色图标；旧图标缓存已清理并重启资源管理器。
- 安装版经 `127.0.0.1:10808` 请求 YouTube 返回 HTTP 200。

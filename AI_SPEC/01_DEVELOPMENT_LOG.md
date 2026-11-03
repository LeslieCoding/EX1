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

## v0.1.3 清亮绿色调整
- 根据实际界面反馈，将主色由偏深的 `#189162` 提亮为 `#35B97F`，同步提高 SVG/PNG/ICO 高光与渐层亮度。
- 保持低饱和浅翡翠绿，不使用荧光绿；危险操作仍保留独立红色语义。
- 版本号同步升至 0.1.3；未改动任何连接、订阅或系统代理逻辑。
- `cargo test` 3/3、`pnpm web:build`、`pnpm tauri build` 均通过。
- v0.1.3 NSIS `/S` 静默安装返回 0；真实窗口、桌面、exe 和任务栏截图均确认清亮绿色图标。
- 安装版经 `127.0.0.1:10808` 请求 YouTube 返回 HTTP 200。

## v0.1.4 浓郁透亮翡翠绿
- 根据用户纠正，将“亮”与“浅”区分处理：主色由 `#35B97F` 调整为更饱和、更透亮的 `#00C271`，不再通过兑白提亮。
- 图标渐层调整为 `#18D98B → #00C271 → #00AC60`，保留明暗层次和立体质感。
- 版本号同步升至 0.1.4；未改动任何连接、订阅或系统代理逻辑。
- `cargo test` 3/3、`pnpm web:build`、`pnpm tauri build` 均通过。
- v0.1.4 NSIS `/S` 静默安装返回 0；真实窗口、桌面、exe 和任务栏截图均确认浓郁透亮绿色。
- 安装版经 `127.0.0.1:10808` 请求 YouTube 返回 HTTP 200。

## v0.1.5 金属玻璃图标
- 参照 LeslieMusic 实际 `logo.png` 的乳白主体、银绿金属包边、冷色高光与右下厚度阴影，重制 EX1 标志；绿色 `#00C271` 和界面配色不变。
- 新增 `src/ex1-mark.png` 作为界面左上角同源金属图标，SVG 源同步加入金属渐变与厚度层；Windows PNG/ICO 全套更新。
- 版本号同步升至 0.1.5；未改动 Xray、订阅、节点选择和系统代理逻辑。
- `cargo test` 3/3、`pnpm web:build`、`pnpm tauri build` 均通过。
- v0.1.5 NSIS `/S` 静默安装返回 0；真实窗口、桌面、exe 与任务栏 PrintWindow 截图均确认金属图标和旧缓存已更新。
- 安装版经 `127.0.0.1:10808` 请求 YouTube 返回 HTTP 200。

## v0.1.6 奶白背景与订阅协议兼容
- 背景改为 `#FFFEFC → #FCFBF7` 的净白高光奶白渐变，卡片使用净白反光层，去除灰黄感；绿色与金属图标不变。
- 订阅解析由仅 VLESS 扩展为 VLESS、VMess、Hysteria2；VLESS/VMess 走 Xray，Hysteria2 走内嵌 mihomo v1.19.33，统一监听 `127.0.0.1:10808`。
- mihomo 二进制 SHA256：`0b4c7353057a78de8a09db445351aadec7d72a4f48adc7d99691f3c5ff74d3da`。
- 修复 URI fragment 未 URL 解码导致中文/emoji 节点名显示为 `%E5...` 的问题，新增对应回归测试；`cargo test` 5/5 通过。
- 真实 UI 添加 KTM001 成功，显示 27 个正常中文节点；Hysteria2 `SG新加坡-1` 与 VMess `SG新加坡-02` 分别实测 YouTube HTTP 200。
- 最终窗口截图确认奶白背景、KTM001 正常节点名及当前 VMess 连接，包内核心路径核验通过。
- Windows Defender 对 `%LOCALAPPDATA%\EX1` 的 exe 报 `Behavior:Win32/Impact.A!ml` 并隔离；未关闭 Defender、未添加排除项。同一 NSIS 包以 `/S /D=D:\Software\EX1\Package` 安装到 D 盘后正常运行，桌面与开始菜单已指向该正式版。
- 安装包 SHA256：`157f42f8c6de9a7deaf097c7d4de142e8d7d507486867e3f46142e83d5911447`。

## EX1 与 ProxyOne 同节点三轮测速
- 条件：同一 EdgeTunnel 订阅，固定实际出口 `104.26.0.175:2083`，独立测试内核避免自动刷新干扰；同一 GitHub 官方 Xray ZIP，单次 15,029,603 字节，顺序 EX1→ProxyOne、ProxyOne→EX1、EX1→ProxyOne。
- 第 1 轮：EX1 2.700 MB/s，ProxyOne 0.464 MB/s。
- 第 2 轮：ProxyOne 0.647 MB/s，EX1 0.325 MB/s。
- 第 3 轮：EX1 0.514 MB/s，ProxyOne 0.425 MB/s。
- 中位数：EX1 0.514 MB/s，ProxyOne 0.464 MB/s；EX1 快约 10.6%，三轮 2 胜 1 负。
- 平均数：EX1 1.180 MB/s，ProxyOne 0.512 MB/s；EX1 首轮峰值显著拉高平均数，因此只下“本轮 EX1 略快”的结论，不宣称绝对性能领先。

## v0.1.7 无断连并行选优
- 旧自动选择会逐节点停止并重启当前核心，造成界面卡顿和当前连接中断；改为独立端口测速，最多 4 路并行，测速阶段不修改当前节点、不重启当前核心。
- 测速客户端统一使用 SOCKS5，修正 Xray 入站与旧 HTTP 代理地址不匹配导致的 VMess 探测失败。
- 仅在最佳节点确实变化时切换一次；最佳节点不变则不重启。订阅信息节点不参与选优，关闭自动选择后不应用过期测速结果。
- 当前订阅保持每 15 分钟自动更新并在开启自动选择时重新选优；全部订阅仍为手动并行更新。
- 界面明确显示“后台并行测速不会中断当前连接”。
- `cargo test` 5/5、`pnpm web:build`、`pnpm tauri build` 均通过；真实 KTM001 27 节点页面完成多轮选优，测速成功节点随实时可达性变化，结束后无残留 probe 配置或进程，YouTube HTTP 200。
- 安装包继续使用 `/S /D=D:\Software\EX1\Package` 安装，包内核心运行正常。
- 安装包 SHA256：`b1a2234e9e14b570fa3f185bb0e7fda982caba9d1be7b38b53f1332b05240e4b`。

## v0.1.8 隐藏自启、静默启动与 TUN
- 新增开机自启动、静默启动、TUN 全局模式三个按钮及持久化状态。
- 自启动采用 Hidden 计划任务＋无窗口 VBS，不写 Run 键或启动文件夹；普通自启与静默自启通过预置控制任务切换，不反复创建删除任务。
- 静默启动只运行核心，不打开主窗口；正常打开 EX1 时停止静默任务并接管核心。
- TUN 使用独立 Hidden＋Highest 的 `EX1TunWorker`，开启时停止系统代理，关闭时经 `EX1TunStop` 停止管理员核心并恢复原核心与代理。
- 所有 `schtasks` 调用增加 `CREATE_NO_WINDOW`，修复按钮点击时控制台连续闪烁；系统中文错误按 GBK 正确解码。
- 启动控制操作串行执行，避免普通自启/静默自启异步覆盖；TUN DNS 使用 1054，避免与现有 ProxyOne 的 1053 冲突。
- `cargo test` 5/5、`pnpm web:build`、`pnpm tauri build` 均通过。
- 实测：两个启动任务均 Hidden，Run 键和启动文件夹无 EX1；静默启动不显示主窗口且 HTTP 200；TUN 建立 Meta 接口且直连 HTTP 200；关闭 TUN、正常重开 EX1 后均恢复 HTTP 200。
- 安装包继续使用 `/S /D=D:\Software\EX1\Package` 安装，SHA256：`183a2a76088e2be5a90e256b9df047da55af3800e19427e926cb0914ea510650`。

## v0.1.9 自动选优提速
- 自动选优并发由 4 路提高到 12 路，独立端口扩展为 `10820..10831`；保留每节点最多两次真实 HTTP 探测、单次 4 秒超时、最佳延迟判断和仅节点变化时切换，未减少检测项目。
- v0.1.8 同一 EdgeTunnel 22 节点真实窗口按钮基线为 18.755 秒；v0.1.9 三轮分别为 6.304、6.004、10.004 秒，首轮减少 66.4%，新版三轮中位数为 6.304 秒。
- 测速期间实测 12 个 Xray probe 进程同时运行；当前 `127.0.0.1:10808` 由原有 mihomo PID 31984 提供服务，选优前后未重启，YouTube `generate_204` 持续返回 HTTP 204。
- 每轮结束后 probe 进程和 probe 配置文件均为 0；真实窗口截图确认按钮恢复、最佳节点与延迟结果正常更新，无错误提示。
- `cargo fmt --check`、`cargo test` 5/5、`pnpm web:build`、`pnpm tauri build` 均通过；安装版 ProductVersion/FileVersion 均为 0.1.9。
- 安装包大小 30,603,760 字节，SHA256：`bd56bf992bd4d254fdc5af87ca8c7466b5dfc608d9b7f1bec85bdd151c0cee67`。

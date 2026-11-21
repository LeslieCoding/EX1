# EX1 开发与验收记录

## v0.4.3 提高订阅与节点刷新频次
- URL 订阅默认自动更新由 60 分钟改为 15 分钟，旧默认 60 分钟通过 v3 标记一次性迁移，自定义其它周期保持不变。
- Windows 独立后台全量选优由 6 小时改为 15 分钟；与订阅周期同时到期时，两项均执行，不再跳过订阅更新。
- `cargo test` 29/29、前端构建和 NSIS Release 构建通过。Windows 0.4.2 已覆盖安装 0.4.3，付费订阅节点保留，多轮 Google 搜索与百度请求均返回 HTTP 200。
- 本地安装包 `EX1_0.4.3_x64-setup.exe`：49,896,360 字节，SHA256 `6ce7a0a8fc3684a29594bae58a9554a204bd2fa0762716c9835a1c7481d3a5c6`；桌面副本 `D:\Leslie\Desktop\EX1_v0.4.3_x64-setup.exe`。


## 架构
- Tauri 原生窗口 + Rust 后端 + 原生 HTML/CSS/TypeScript 前端。
- Xray 26.10.21（官方安全更新版），打包于 `src-tauri/sidecar/xray.exe`，运行时不依赖 v2rayN 路径。
- 配置使用新版 `socks` 入站和 `websocket` 传输字段，不依赖 GeoIP/GeoSite 数据文件。
- 订阅 YAML 与 VLESS 链接解析、并行全部更新、全部订阅后台定时刷新、当前订阅自动选优。
- 单个 mihomo 核心并发真实测速，兼容 VLESS、VMess、Hysteria2；自动选择开启时前后端均禁止手选。

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

## v0.2.0 系统托盘与自动恢复
- 关闭按钮改为只隐藏主窗口，不再停止核心和系统代理；新增与桌面一致的绿色托盘图标，单击恢复窗口，右键菜单提供“显示 EX1”和“退出（1 分钟后自动恢复）”。
- 新增命名互斥体单实例；隐藏窗口时再次启动会恢复原进程，不重复启动核心。恢复时明确取消全屏、最大化和最小化，配置默认普通窗口 1100×760。
- 参照 ProxyOne 实际 `WindowsNetworkGuard`（Hidden、每 10 分钟、隐藏 VBS）实现 `EX1Watchdog`，缩短为每 1 分钟检查；VBS 只在 `ex1.exe` 不存在时启动应用，任务使用当前用户交互权限，不写 Run 键。
- 实测关窗后进程保持、窗口不可见、代理 HTTP 204；再次启动和真实托盘图标点击均恢复原 PID，进程总数为 1，窗口未最大化。
- `EX1Watchdog` 实测 Hidden、间隔 `PT1M`、动作指向无窗口 VBS；强制结束 PID 28236 后 8.1 秒自动拉起 PID 22184，`LastTaskResult=0`，恢复后 HTTP 204。托盘右键菜单已真实截图确认。
- `cargo fmt --check`、`cargo test` 5/5、`pnpm web:build`、`pnpm tauri build` 均通过；安装版 ProductVersion/FileVersion 均为 0.2.0。
- 安装包大小 30,705,380 字节，SHA256：`a0bcb00901efa0e2202725d7567bccbe91f122d088e18321ce20a902b9d21b90`。

## v0.2.1 开机静默启动修复与全功能回归
- 查明 v0.2.0 的 `EX1SilentStart` 登录时返回 1：旧 VBS 同时存在嵌套引号错误、不可用 WMI 启动对象，并且只启动核心、不启动 EX1 托盘。改为通过 `Win32_Process` 和 `Chr(34)` 启动 EX1 本体 `--silent`，返回 WMI 真实状态码；普通启动共用同一可靠链路。
- 静默启动现在保留 EX1 进程、核心、单实例和系统托盘，只隐藏主窗口；最终安装版重放 `EX1SilentStart` 返回 0，PID 24040 以 `--silent` 存活、窗口隐藏，包内核心运行且代理 HTTP 204。真实通知区找到 EX1 并点击恢复同一 PID，随后关窗仍保持代理。
- 核心启动增加 10808 监听健康检查；TUN 开启等待核心就绪并停用系统代理，关闭先等待旧 TUN 端口释放再启动新核心，避免把旧监听误判为新核心成功。实测 Meta 网卡上线、系统代理停用、直连 HTTP 204；关闭后 EX1TunWorker 停止、Xray 恢复、系统代理恢复且 HTTP 204。
- 自动测速由最多 12 个独立核心改为 1 个 mihomo＋32 路并发，同一节点两次探测并发取先成功结果。EdgeTunnel 22 节点三轮为 5.013、4.391、5.026 秒；KTM001 的 Hysteria2 与 VMess 均取得真实延迟，自动切换后 HTTP 204。
- 自动选择开启时应用启动立即使用已有节点选优；后台每 15 分钟并行更新全部订阅，仍只重选当前订阅。手动全部更新实测两条订阅共 49 节点、中文名称正常、无错误。
- 按用户要求停用 ProxyOne 的 `WindowsNetworkGuard` 与 `WindowsProxyService`，EX1 保持 `EX1SilentStart` 和 `EX1Watchdog` 为 Hidden＋Ready，`EX1Autostart` 在静默模式下保持 Disabled。
- `cargo fmt --check`、`cargo test` 5/5、`pnpm web:build`、`pnpm tauri build` 均通过；NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.2.1。最终窗口截图确认开关状态、当前节点、核心运行和无错误横幅。
- 安装包大小 30,706,417 字节，SHA256：`740041fffebc935dc793864dc5a9239d66473e72305ca1d9f6fd80b4437f6965`。

## v0.2.2 残留核心与异常退出恢复
- App 启动、启动/切换代理、停止代理和托盘退出统一清理 EX1 残留核心；通过 Windows 进程快照、完整可执行文件路径和父子关系识别，不按进程名误伤其他代理软件。常规重启保留当前实例的测速子进程，冷启动则清理上一实例的全部核心。
- 进程结束后使用带 `SYNCHRONIZE` 权限的句柄等待真实退出，避免把已结束的残留核心误报为“未及时退出”。
- 系统代理恢复不再把异常退出后的 EX1 地址当成原始设置；缺失或无效的恢复快照回退为关闭代理，停止后不会继续指向失效的 `127.0.0.1:10808`。
- `cargo fmt`、`cargo test` 7/7、`pnpm web:build`、`pnpm tauri build` 均通过；新增核心路径归一化和测速子进程保留两项回归测试。
- 最终安装版分别制造 Mihomo 与 Xray 残留进程实测：Mihomo `31180`、Xray `11052` 均在 App 启动后退出，新 Xray `5352` 由新 App 进程接管；代理请求为 HTTP 204，界面无错误提示。
- 停止代理后 EX1 核心为 0，`10808` 的 TCP/UDP 监听均为 0，系统代理恢复关闭；再次启动后 HTTP 204。安装版 ProductVersion/FileVersion 均为 0.2.2，NSIS `/S` 安装返回 0。
- 安装包大小 30,718,318 字节，SHA256：`0128aad63fb8134f8aadf659a16992528c99af9998544c37d963a04b98a98021`。

## v0.2.3 Cloudflare 优选节点筛选修复
- EdgeTunnel 订阅更新后节点统一命名为“CF电信优选1～22”，旧规则因“电信”二字排除全部节点，导致“当前订阅没有可自动选择的境外节点”；Cloudflare 的“电信/官方优选”现按代理入口处理，不再误判为国内落地。
- 地区判断前剔除“国内官网/中国官网”说明文字，避免香港、新加坡节点仅因附带官网文字被排除；“上海”“电信上海”“CF上海节点”等真实大陆地区名称仍保持过滤。
- `cargo fmt --check`、`cargo test` 7/7、`pnpm web:build`、`pnpm tauri build` 全部通过；NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.2.3。
- 最终安装版真实 EdgeTunnel 22 节点全部参与测速，真实界面“立即选优”4.204 秒完成并选择当时最低延迟的“CF电信优选5”（753 ms）；原错误横幅消失，代理请求 HTTP 204，最终窗口截图确认无错误提示。
- 安装包大小 30,711,964 字节，SHA256：`a87031ca6af3a1ed064352e205ba948fee8406dd16a9e05cb0084cd4918cbaf6`。

## v0.3.0 运行审计、持久化日志与恢复能力
- 新增按天持久化日志：`%APPDATA%\EX1\logs` 记录应用操作、异常、Xray、Mihomo 和测速核心输出；记录前 3 个真实测速失败原因，应用日志与错误横幅隐藏完整订阅 URL，未将订阅 token 写入日志。
- 流量、重置和到期信息项不再作为默认节点或可手选节点；新增默认节点回归测试，KTM 切换时实际默认香港节点而非“剩余流量”。
- 自动选优增加进程内串行锁，防止手动与后台任务争用 10820/10821；测速结果绑定订阅 ID，切换订阅或刷新删除节点后忽略过期结果，避免覆盖新订阅。
- 测速核心改用独立 `%APPDATA%\EX1\probe` 数据目录，消除主 Mihomo 与测速核心共用 `cache.db` 产生的 `can't open cache file: timeout`；最终两轮测速均无新增缓存锁警告，临时配置无残留。
- 正式版只使用安装包内核心，开发目录仅限调试构建；从项目目录启动安装版也能正确清理旧核心，不再误报 10808 被占用。
- 订阅下载在当前代理失败或返回无效内容时自动直连重试；实测当前 KTM VMess 节点失效时，EdgeTunnel 仍通过直连更新成功。单条订阅失败显示订阅名称并保留原节点，不再显示内部 ID，不再被后续选优成功清除；前端不再同时弹“全部更新成功”。
- KTM001 服务端实测直连与代理均为 HTTP 200、正文 0 字节，缓存 Hysteria2 节点请求 502、VMess 节点请求 503，22 个境外节点测速全部失败；判定为外部订阅/节点服务异常，客户端保留 27 个缓存节点并给出明确提示，未伪装修复成功。EdgeTunnel 不受此影响。
- 后端状态变化通过 `ex1-state` 实时推送到界面；启动、看门狗、系统代理和核心错误均写入日志，未处理 panic 也会落盘。
- `cargo fmt --check`、`cargo test` 10/10、`pnpm web:build`、`pnpm tauri build` 全部通过；NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.3.0。
- 最终安装版 EdgeTunnel 自动选优 3.711 秒并恢复 HTTP 204；TUN 开启后 Meta 网卡上线、系统代理停用、直连 HTTP 204，关闭后系统代理和原核心恢复、HTTP 204。`EX1SilentStart` 重放结果 0，进程以 `--silent` 驻留、主窗口未创建、HTTP 204，再次启动复用同一 PID。
- 最终普通窗口无错误横幅，EdgeTunnel 22 节点测速全部成功，自动选择 CF电信优选17（686 ms），代理 HTTP 204；日志核验无订阅 token。
- 安装包大小 30,730,172 字节，SHA256：`78ad9792c02dd119c90521d94fbd46078423d91e5e893ba576eeda44df735f09`。

## v0.3.1 关闭按钮隐藏修复
- 复现确认：原生 `WM_CLOSE` 前后窗口均为 `VISIBLE=True`、`CLOAKED=0`，旧版在 `CloseRequested` 回调内同步调用 `hide()`，隐藏动作被 Windows/WebView 关闭流程覆盖，导致点击关闭按钮没有反应。
- 修复为关闭事件先 `prevent_close()`，等待 100ms 关闭流程结束后再异步隐藏主窗口；隐藏结果和错误写入持久化日志，不再静默忽略。
- 最终安装版实测：关闭前 `VISIBLE=True`，关闭后 `VISIBLE=False`，进程存活且代理 HTTP 204；日志记录“收到主窗口关闭请求”和“主窗口隐藏到托盘”。再次启动恢复同一 PID、`VISIBLE=True`，未重建进程。
- `cargo fmt --check`、`cargo test` 10/10、`pnpm web:build`、`pnpm tauri build` 均通过；NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.3.1。
- 安装包大小 30,738,973 字节，SHA256：`32815ab5047796c3dafd4e87c634ffcd729bc916e2fd66b464b6178fa932a723`。

## v0.3.2 后台刷新降噪与状态事件修复
- 运行日志显示 KTM001 因外部空响应每 15 分钟失败一次；旧逻辑因此跳过当前订阅自动选优，并重复显示错误横幅。后台刷新现使用静默结果：失败仍完整写日志，非当前订阅失败不更新界面横幅，同时继续为当前 EdgeTunnel 选优；手动“更新全部订阅”仍显示全部错误和“操作未完全成功”。
- 新增 `should_refresh_after_update` 回归测试，锁定“后台部分失败仍刷新当前订阅、手动失败不自动覆盖错误”的行为；`cargo test` 11/11 通过。
- 实际复现后台状态推送失败：Tauri capability 未允许 `core:event:allow-listen`，且 `removeUnusedCommands=true` 把前端动态使用的 `listen` 命令裁掉，先后报 `event|listen not allowed by ACL` 和 `Command listen not found`。新增最小 capability，并关闭未使用命令裁剪。
- 前端改为先完成 `ex1-state` 事件订阅，再读取初始状态，避免启动自动选优在监听器注册前完成、界面等待 15 秒轮询才追上。
- 最终安装版启动后界面无错误，当前节点与核心日志一致为 CF官方优选4（768 ms），自动选择开启，代理 HTTP 204。
- `cargo fmt --check`、`cargo test` 11/11、`pnpm web:build`、`pnpm tauri build` 全部通过；NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.3.2。
- 安装包大小 31,387,234 字节，SHA256：`979c42e23c97043a93f1431a77acda0a0a64ed523eb80b20e7fda926515e58a5`。

## v0.3.3 关闭按钮原生隐藏修复
- 用户再次报告右上角关闭无效后实测复现：关闭事件已收到，Tauri `hide()` 当时也返回成功并写入“隐藏到托盘”，但 Win32 `IsWindowVisible` 在 100～2200ms 全部仍为 `True`，证明旧实现只是在日志里“假成功”。
- 修复为关闭流程结束后同时调用 Tauri `hide()` 和原生 `ShowWindow(SW_HIDE)`，以 `IsWindowVisible` 连续 3 次确认真实隐藏；若 Windows 再次显示窗口则自动重试最多 10 次并写错误日志，不再把 API 返回 Ok 当成隐藏成功。
- 最终安装版真实验收：关闭前 `VISIBLE=True`；关闭后 100ms 仍为过渡态，300/600/1000/1600/2200ms 均为 `VISIBLE=False`；进程保持存活，代理 HTTP 204。再次启动恢复同一 PID、窗口重新可见，代理仍为 HTTP 204。
- 日志已确认输出“关闭请求已处理，主窗口原生隐藏到托盘”；v0.3.1 的 100ms 延后关闭流程修复保留，不做回退。
- `cargo fmt --check`、`cargo test` 11/11、`pnpm web:build`、`pnpm tauri build` 全部通过；NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.3.3。
- 安装包大小 31,382,957 字节，SHA256：`cdceedf5d272a74f04fab607610b3114b1393d7e62cb221e0be99b4c72a19307`。

## v0.3.4 自启任务自举、anytls 与 CI 出包
- 全新安装不再依赖预置计划任务：`startup.rs` 改为启动时自动创建 `EX1Autostart`/`EX1SilentStart`（Hidden、登录触发、电池两项限制关闭、ExecutionTimeLimit=PT0S、动作指向 `%APPDATA%\EX1\{normal,silent}-start.vbs`），并按持久化状态同步启用项；旧的 `EX1TaskNormal`/`EX1TaskSilent`/`EX1TaskDisable` 控制任务改为自动删除。实测删除两个自启任务后重启应用自动重建，状态保持 静默=Ready、普通=Disabled。
- `set_auto_start`/`set_silent_start` 在创建任务失败时回滚持久化状态并返回错误，修复“界面显示已开启但实际未生效”。
- 新增 anytls 协议：YAML 与 `anytls://` 链接解析支持 ALPN、SNI、skip-cert-verify，运行期由内置 mihomo 承载（与 Hysteria2 走同一路径），自动测速与 TUN 复用同一代理渲染。用 mihomo 自带 anytls 监听器（`127.0.0.1:12443` + 自签证书）端到端验证：经 EX1 代理端口访问 baidu / api.github.com 均返回 200，服务端日志确认会话建立。
- 新增 `.github/workflows/build.yml`：windows-latest 上安装依赖、前端构建、`cargo test`、`pnpm tauri build` 并上传 NSIS 产物；本机没有 Rust/Node/MSVC，编译全部在 CI 完成。
- CI run 41999590125：依赖安装、前端构建、`cargo test`（含新增 anytls、任务 XML、开关组合用例）全部通过，NSIS 出包成功。
- 安装包 31,437,785 字节，SHA256：`d0219fa25aad2710507f44428483d87afbe205cf7baa751ac984cf24a41464a8`；`/S /D=D:\SoftWare\EX1\Package` 静默安装返回 0，ProductVersion/FileVersion 均为 0.3.4。
- 真机回归：安装后系统代理保持 `127.0.0.1:10808`、YouTube 204；`EX1Autostart`(Disabled)、`EX1SilentStart`(Ready)、`EX1TunWorker`、`EX1Watchdog` 全部 Hidden；日志输出“开机启动任务已同步：普通=false，静默=true”。
- Release v0.3.4 与安装包已发布：`https://github.com/LeslieCoding/EX1/releases/tag/v0.3.4`。

## v0.3.6 端口与订阅更新间隔可配置
- 代理端口不再写死 10808：`PersistedState.proxy_port` 持久化，核心（Xray/Mihomo/TUN）、系统代理、启动清理、托盘退出、端口占用检查全部改用配置端口；界面新增「代理端口」输入框（1024-65535，10820/10821 保留给测速核心）。切换端口时先恢复原系统代理设置、重启核心、再按新端口启用代理；核心启动失败会回滚端口并恢复原端口。
- 系统代理快照新增 `ex1_server` 字段：进程被强杀后换端口启动时，不再把上一实例残留的 `127.0.0.1:<旧端口>` 当成“原始代理设置”，改为复用旧快照，避免停代理后还原到失效地址。实测：旧实例残留 10888、改回 10808 启动后，恢复文件仍是 `enabled=0, server=127.0.0.1:7897, ex1_server=127.0.0.1:10808`。
- 订阅自动更新间隔不再写死 15 分钟：`PersistedState.update_interval_min`（1-1440 分钟），后台循环每轮读取新值；界面新增下拉 5/15/30/60 分钟。实测设为 1 分钟后，启动选优 18:14:06、后台第二轮 18:15:19。
- 顺带修正：`set_auto_start`/`set_silent_start` 失败回滚（v0.3.4 已加）保持不变；托盘退出与启动清理改为按配置端口调用。
- CI run 42009417870：前端构建、`cargo test`（新增端口校验、间隔钳制、本机地址识别用例）通过，NSIS 出包成功。
- 安装包 31,450,611 字节，SHA256：`89a52e0931c16dc9ba4d8b058b900b7cc986fd25b732da227e3c22496f1661cd`；`/S /D=D:\SoftWare\EX1\Package` 静默安装返回 0，ProductVersion/FileVersion 均为 0.3.6。
- 真机回归：默认端口仍 10808、YouTube 204；改 10888 启动后 10888 监听、系统代理指向 10888、10808 释放；改回 10808 后恢复 204。UIA 核验界面：`proxy-port` 为 Spinner 值 `10808`、`update-interval` 为 ComboBox 值「每 15 分钟」，均可见且可交互。
- Release v0.3.6 与安装包已发布：`https://github.com/LeslieCoding/EX1/releases/tag/v0.3.6`。

## v0.3.7 订阅自动更新默认 5 分钟
- 按用户要求把订阅自动更新默认间隔由 15 分钟缩短为 5 分钟，档位扩展为 3 / 5 / 10 / 15 / 30 / 60 分钟（`models::default_update_interval`、界面下拉、前端兜底值同步）。
- 本机把已有配置的 `update_interval_min` 一并改为 5，界面实测下拉显示「每 5 分钟」，保存值为 5。
- CI run 42012190943：前端构建与 `cargo test` 通过，NSIS 出包成功。
- 安装包 31,443,440 字节，SHA256：`98e2378f5cdde144691ff313e34ef1604ce76926139aa7e9e6c0bb35aa4ad6ba`；`/S /D=D:\SoftWare\EX1\Package` 静默安装返回 0，ProductVersion/FileVersion 均为 0.3.7。
- 真机回归：安装后端口仍 10808、YouTube 204；窗口截图确认「代理端口 10808」「订阅自动更新 每 5 分钟」，节点列表 22 个 VLESS 延迟正常。
- Release v0.3.7 与安装包已发布：`https://github.com/LeslieCoding/EX1/releases/tag/v0.3.7`。

## v0.3.8 国内直连分流与静默安装修复
- 根因确认：旧 Xray 配置没有任何 routing 规则，Windows 系统代理开启后所有 HTTP/HTTPS（包括百度、腾讯、B 站等国内站点）都被送到境外节点，国内页面因此从直连约 0.18 秒恶化到 2–3 秒；同时当前 EdgeTunnel 节点自身吞吐只有约 60–180KB/s，国内绕路和慢节点叠加造成“国内外都卡”。
- 新增 Xray `routing`：`geosite:private`、`geosite:cn`、`geoip:private`、`geoip:cn` 全部走 `direct`，其余默认走代理；`domainStrategy` 为 `IPIfNonMatch`。Xray 进程显式设置 `XRAY_LOCATION_ASSET` 指向 sidecar，避免安装版找不到分流数据。
- 随包加入 v2fly 官方分流数据：`geoip.dat` SHA256 `eedb24b3b9fcba27e06b92c0428dab90d9496083091ae4cf39e11387fd99d0e2`，`geosite.dat` SHA256 `64c3e8a3cb40a0261edd701d9f07d4ca96af69821bf73d3a3be8509bb200bb9f`。
- Hysteria2 与 TUN 的 Mihomo 配置由 `global` 改为 `rule`，使用 `GEOSITE,private,DIRECT`、`GEOSITE,cn,DIRECT`、`GEOIP,CN,DIRECT`，其余 `MATCH,EX1`；启动前复制分流数据到配置目录。
- 安装器关闭语言选择器；Git Bash 调用 NSIS 时必须设置 `MSYS_NO_PATHCONV=1`，否则 `/S` 会被改写导致安装器进入图形向导。
- 改动 rebase 到远端 v0.3.7 之上，保留 anytls、可配置端口、自启自举与 5 分钟订阅更新；`cargo test` 21/21、`pnpm web:build`、`pnpm tauri build` 通过；安装版 ProductVersion/FileVersion 均为 0.3.8。
- V0.3.8 最终安装版实测（系统代理已指向 `127.0.0.1:10808`）：百度 200/0.112 秒；清华镜像 1MB 1.32 秒、29.7MB/s；YouTube 204/1.79 秒；OVH 1MB 3.74 秒、280KB/s。Xray 日志分别出现 `www.baidu.com`、`mirrors.tuna.tsinghua.edu.cn [socks -> direct]` 与 `www.youtube.com`、`proof.ovh.net [socks >> proxy]`。国内绕路问题已消除，国外速度仍受订阅节点本身波动限制，不作超出节点能力的承诺。
- 自动选优在最终启动时选中 `CF电信优选11`（373ms）。额外实测 22 个 EdgeTunnel 节点 1MB 吞吐：最好约 180KB/s，最差约 25KB/s；KTM001 缓存中的 24 个 Hysteria2/VMess 节点全部无法建连（订阅服务仍返回空内容），判定为上游节点服务不可用，不是客户端路由问题。
- v0.3.8 安装包 33,685,810 字节，SHA256 `f46d963fcd0c8011ec4f969e6aa70eae325869c190828002285754b2716ff2f5`；`MSYS_NO_PATHCONV=1 ... /S /D=D:\Software\EX1\Package` 静默安装成功。`EX1Watchdog` 已恢复启用，应用以 `--silent` 驻留托盘，系统代理保持开启。

## v0.4.0 自动更新
- 新增 `update.rs`：查询公开更新源 `https://api.github.com/repos/LeslieCoding/EX1-updates/releases/latest`（主仓库私有，APP 无法匿名读取 release，因此单独建公开仓库放安装包与说明），按语义化版本比较判断是否有新版本；请求默认复用系统代理（`system_proxy::current_proxy_url`），避免 GitHub 直连被重置。
- 新增命令 `check_update`、`install_update`、`set_auto_update`；`PersistedState` 增加 `auto_update`（默认开）与 `last_auto_update`，`PublicState` 增加 `auto_update` 与 `update` 快照，状态通过 `ex1-state` 实时推送。
- 顶栏新增更新按钮：无更新时显示「检查更新」，有新版本变绿色实心「更新到 vX.Y.Z」，检查中/安装中显示忙碌态并带旋转图标；侧栏新增「自动更新 EX1」开关（说明：启动后与每 6 小时检查并自动安装）。
- 自动安装：下载到 `%TEMP%\EX1-update-setup.exe`（小于 1MB 视为异常），先写 `%APPDATA%\EX1\update-pending.json` 标记目标版本，再拉起无窗口 VBS 助手；助手轮询 PID 等待应用真正退出后执行 `installer /S /D=<安装目录>`，安装完成再启动新版本。启动时若待装版本已不大于当前版本则清除标记。
- 防重复安装：自动安装前同时比对 `update-pending.json` 与 `last_auto_update`，同一目标版本不会自动安装第二次（早期版本曾出现同一版本被装两次）。
- `EX1_UPDATE_FEED` 环境变量可覆盖更新源，用于本机测试与私服镜像。
- 实测：v0.3.9 启动后 8 秒自动检查返回「当前 0.3.9，最新 0.3.9，可用=false」，界面显示「已是最新 v0.3.9」；用本地假更新源（`http://127.0.0.1:18080/latest.json` 声明 v9.9.9）验证完整链路：检查 → 下载 33,785,249 字节 → 启动助手 → 应用退出 → 静默安装 → 新进程启动，日志依次出现「更新检查」「更新包已下载」「更新安装程序已启动」与新的「应用启动」。

## v0.4.1 自动更新防抖修复
- 复现：假更新源声明 v9.9.9、实际安装包为 0.4.0 时，同一个目标版本被自动安装了两次（0.4.0 首轮 09:34:25 装完重启后，09:34:58 又装一次）。
- 修复：`auto_install_allowed` 改为三重判定——`update-pending.json` 的目标版本 + 标记年龄小于 12 小时 + `last_auto_update` 不是同一版本；标记改为**下载前**写入（原先只在拉起安装器前写），并在判定时输出「自动更新防抖」日志，暴露 `标记版本/标记年龄小时/上次尝试`。
- 安装助手 VBS 由固定 `WScript.Sleep 2500` 改为轮询 `Win32_Process` 等待当前 PID 真正退出，再执行 `installer /S /D=<安装目录>`，安装完成延迟 1.2 秒启动新版本。
- 实测（0.4.1 + 假源 v9.9.9）：09:59:07 首轮判定 `标记版本=None` → 写入标记 → 下载 33,769,465 字节 → 拉起助手 → 09:59:23 新进程启动；09:59:38 第二轮判定 `标记版本=Some("9.9.9")、标记年龄小时=Some(0)、上次尝试=Some("9.9.9")` → 拦截，假源只收到 `latest.json`，没有再下载安装包。
- 真实更新源复核：0.4.1 启动 8 秒后返回「当前 0.4.1，最新 0.4.1，可用=false」，UIA 读到顶栏按钮名称为「已是最新 v0.4.1」，`auto-update-toggle`、`update-interval`(每 5 分钟)、`proxy-port`(10808) 均在界面可见可交互。
- 安装包 33,769,465 字节，SHA256 `a7f37bd729fce4bcf9e6b3aa642d29c5ef4304a2352de6e36b06451604779049`；更新源公开仓库 `LeslieCoding/EX1-updates` 的 latest release 已指向 v0.4.1 并带同名安装包。


## Linux 稳定性接管与更新源恢复（2026-11-13）
- 主仓库远端最新提交确认为 v0.4.1 `a01a681`，保留全部 Linux 本地移植与既有订阅。配套公开仓库 `LeslieCoding/EX1-updates` 曾被删除，导致检查更新 404；已重新建立公开仓库并发布同版本官方 Windows 安装包，Release API 实测 HTTP 200。
- 例行订阅刷新与节点选优、断网救火完全分离：订阅默认每 5 分钟更新；联网看门狗每 20 秒经当前代理请求真实 HTTPS，连续 2 次失败立即刷新订阅并重新选优，冷却 5 分钟、恢复宽限 60 秒；健康时每 2 分钟独立选优。
- 订阅刷新对比当前节点完整连接配置，未发生变化时不重启核心；测速延迟不参与比较，避免同名节点仅因旧延迟被清空而误重启。后台订阅刷新先完成选优，连接参数或最佳节点变化时最后最多重启一次，不再刷新前、选优后连续重启。订阅更新增加串行锁，配置保存改为临时文件原子替换。
- Linux 系统代理开关显示 GNOME 实际 `mode` 和地址，前台每 2 秒同步；不再用保存的 `proxy_enabled` 冒充系统真实状态。开启系统代理失败会停止刚启动的核心，避免留下界面外的隐蔽代理。
- systemd 自启迁移到 `graphical-session.target`，保留异常退出 5 秒拉起和应用内 10 秒核心看门狗。新增 `scripts/build-linux.sh`，固定使用项目内 Node/pnpm/Cargo 环境与 Rust 1.95。
- 验证：前端 TypeScript/Vite 构建通过；Rust 21 项测试通过（含延迟变化不重启、连接变更才重启）；正式 DEB 构建成功。备用端口 38008 真实 HTTPS 204，旧代理 37993 同时保持 204；杀崩溃备内核后自动拉起并恢复 204；日志确认 2 分钟独立选优执行。正式切换采用双跑后切 GNOME，最终只保留 EX1 与 mihomo，GNOME 为 manual/127.0.0.1:38008，真实 HTTPS 204。最终版 03:50 后台刷新日志确认选优前零重启、选优后仅一次重启，节点不变的周期不重启。未进行整机重启验收。

## 更新仓库合并（2026-11-16）
- 将 `LeslieCoding/EX1-updates` 合并回 `LeslieCoding/EX1`，不再维护独立更新仓库；主仓库改为公开，以保留应用匿名检查和下载更新的能力。
- Windows 与 Linux 默认更新地址统一为 `https://api.github.com/repos/LeslieCoding/EX1/releases/latest`。
- 原更新仓库 v0.4.1 安装包与主仓库 Release 安装包均为 33,769,465 字节，SHA256 均为 `a7f37bd729fce4bcf9e6b3aa642d29c5ef4304a2352de6e36b06451604779049`，无需重复上传。

## v0.4.2 Windows 后台请求降噪
- Windows 普通后台订阅刷新与全量选优解耦：当前节点有效且完整连接配置不变时只更新订阅；当前节点失效或连接配置变化时立即全量重选，并在需要时最多重启一次。应用启动、手动“立即选优”、手动更新订阅仍执行完整测速；Linux 专属联网看门狗与 2 分钟选优逻辑未修改。
- 新增每 6 小时独立后台全量选优，调度与订阅更新间隔分离。默认订阅更新间隔由 5 分钟改为 60 分钟，界面提供 15/30/60/120/360 分钟档位；旧配置仅对未迁移的 5 分钟值一次性升级为 60 分钟并写入迁移标记，其它配置保持不变。
- `cargo fmt --check`、`cargo check`、`cargo test` 29/29、`pnpm run web:build` 与 `pnpm tauri build` 全部通过。新增测试覆盖有效节点跳过全量选优、失效/配置变化恢复、6 小时到期判断、手动与故障恢复兼容，以及旧配置迁移和自定义配置保留。
- 本机 NSIS `/S` 安装返回 0，安装版 ProductVersion/FileVersion 均为 0.4.2。真实旧配置从 5 分钟迁移为 60 分钟；现有 EdgeTunnel 订阅 19 个节点启动选优成功，手动“立即选优”实测 19/19 成功且最佳节点不变、不重启核心，代理请求 YouTube `generate_204` 为 HTTP 204、首页为 HTTP 200。
- 真实 EdgeTunnel 在临时 1 分钟后台周期中因连接配置变化触发恢复选优，符合失效/变化恢复策略。另用本机静态订阅受控验证普通周期：16:45:58 日志依次记录“全部订阅更新完成，当前节点未变化时不重启核心”和“后台订阅刷新未触发全量选优：当前节点仍有效且连接配置未变化”，期间没有“开始自动选优”。6 小时低频调度由单元测试验证，未实际等待 6 小时；测试后真实订阅、60 分钟间隔和可用代理均已恢复，临时服务器与文件已清理。
- 安装包：`src-tauri/target/release/bundle/nsis/EX1_0.4.2_x64-setup.exe`，49,900,289 字节，SHA256：`04c6376773aa36209bcd07a69fb4f97c16fded6988411d3b9e7cf4b7bbbdf842`。
- 已提交 `7c7f6c2` 并发布 `https://github.com/LeslieCoding/EX1/releases/tag/v0.4.2`；Release 资产大小与 SHA256 均与本地一致，latest 已指向 v0.4.2。GitHub Actions run `44051797856` 的前端、Rust 测试和 NSIS 构建全部通过。

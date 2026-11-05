# EX1 开发与验收记录

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

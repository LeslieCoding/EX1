import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface ProxyNode {
  name: string
  server: string
  port: number
  protocol: string
  delay: number | null
}

interface Subscription {
  id: string
  name: string
  url: string
  nodes: ProxyNode[]
}

interface AppState {
  subscriptions: Subscription[]
  current_subscription: string | null
  current_node: string | null
  auto_select: boolean
  proxy_enabled: boolean
  auto_start: boolean
  silent_start: boolean
  tun_enabled: boolean
  running: boolean
  last_error: string | null
}

type CommandArgs = Record<string, unknown>

function byId<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id)
  if (!element) throw new Error(`Missing element: ${id}`)
  return element as T
}

const elements = {
  appStatus: byId<HTMLDivElement>('app-status'),
  appStatusText: byId<HTMLSpanElement>('app-status-text'),
  subscriptionSummary: byId<HTMLSpanElement>('subscription-summary'),
  subscriptionCount: byId<HTMLSpanElement>('subscription-count'),
  subscriptionList: byId<HTMLDivElement>('subscription-list'),
  nodeList: byId<HTMLDivElement>('node-list'),
  nodeCount: byId<HTMLSpanElement>('node-count'),
  nodesSubtitle: byId<HTMLParagraphElement>('nodes-subtitle'),
  currentNodeName: byId<HTMLHeadingElement>('current-node-name'),
  currentNodeIndicator: byId<HTMLSpanElement>('current-node-indicator'),
  currentSubscriptionName: byId<HTMLSpanElement>('current-subscription-name'),
  connectionBadge: byId<HTMLSpanElement>('connection-badge'),
  proxyToggle: byId<HTMLButtonElement>('proxy-toggle'),
  proxyToggleLabel: byId<HTMLSpanElement>('proxy-toggle-label'),
  proxyHint: byId<HTMLParagraphElement>('proxy-hint'),
  autoSelectToggle: byId<HTMLInputElement>('auto-select-toggle'),
  autoStartToggle: byId<HTMLInputElement>('auto-start-toggle'),
  silentStartToggle: byId<HTMLInputElement>('silent-start-toggle'),
  tunToggle: byId<HTMLInputElement>('tun-toggle'),
  autoLockNote: byId<HTMLDivElement>('auto-lock-note'),
  addSubscription: byId<HTMLButtonElement>('add-subscription'),
  updateAll: byId<HTMLButtonElement>('update-all'),
  updateAllLabel: byId<HTMLSpanElement>('update-all-label'),
  refreshBest: byId<HTMLButtonElement>('refresh-best'),
  refreshBestLabel: byId<HTMLSpanElement>('refresh-best-label'),
  errorBanner: byId<HTMLDivElement>('error-banner'),
  errorTitle: byId<HTMLElement>('error-title'),
  errorMessage: byId<HTMLSpanElement>('error-message'),
  errorRetry: byId<HTMLButtonElement>('error-retry'),
  errorClose: byId<HTMLButtonElement>('error-close'),
  toast: byId<HTMLDivElement>('toast'),
  toastMessage: byId<HTMLSpanElement>('toast-message'),
  addDialog: byId<HTMLDialogElement>('add-dialog'),
  addForm: byId<HTMLFormElement>('add-subscription-form'),
  addName: byId<HTMLInputElement>('subscription-name'),
  addUrl: byId<HTMLInputElement>('subscription-url'),
  addFormError: byId<HTMLDivElement>('add-form-error'),
  confirmAdd: byId<HTMLButtonElement>('confirm-add'),
  confirmAddLabel: byId<HTMLSpanElement>('confirm-add-label'),
  deleteDialog: byId<HTMLDialogElement>('delete-dialog'),
  deleteSubscriptionName: byId<HTMLElement>('delete-subscription-name'),
  confirmDelete: byId<HTMLButtonElement>('confirm-delete'),
  confirmDeleteLabel: byId<HTMLSpanElement>('confirm-delete-label'),
}

let appState: AppState | null = null
let initialLoading = true
let stateLoading = false
let stateRequestId = 0
let busy: string | null = null
let localError: string | null = null
let localErrorTitle = '操作未完成'
let dismissedBackendError: string | null = null
let pendingDeleteId: string | null = null
let toastTimer: number | undefined

function currentSubscription(): Subscription | null {
  if (!appState?.current_subscription) return null
  return appState.subscriptions.find(({ id }) => id === appState?.current_subscription) ?? null
}

function currentNodes(): ProxyNode[] {
  return currentSubscription()?.nodes ?? []
}

function canInteract(): boolean {
  return !initialLoading && appState !== null && busy === null
}

function createIcon(symbol: string): SVGSVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
  svg.setAttribute('class', 'icon')
  svg.setAttribute('aria-hidden', 'true')
  const use = document.createElementNS('http://www.w3.org/2000/svg', 'use')
  use.setAttribute('href', `#${symbol}`)
  svg.append(use)
  return svg
}

function createButton(label: string, action: string, className = 'icon-button small'): HTMLButtonElement {
  const button = document.createElement('button')
  button.type = 'button'
  button.className = className
  button.dataset.action = action
  button.textContent = label
  return button
}

function formatError(error: unknown): string {
  if (typeof error === 'string') return error
  if (error instanceof Error) return error.message
  if (typeof error === 'object' && error !== null && 'message' in error) {
    return String((error as { message: unknown }).message)
  }
  return String(error || '未知错误，请稍后重试')
}

function showToast(message: string): void {
  window.clearTimeout(toastTimer)
  elements.toastMessage.textContent = message
  elements.toast.hidden = false
  toastTimer = window.setTimeout(() => {
    elements.toast.hidden = true
  }, 3200)
}

function showError(message: string, title = '操作未完成'): void {
  localError = message
  localErrorTitle = title
  renderError()
}

function renderError(): void {
  const backendError = appState?.last_error ?? null
  const showBackendError = backendError !== null && backendError !== dismissedBackendError
  const message = localError ?? (showBackendError ? backendError : null)
  elements.errorBanner.hidden = message === null
  if (message === null) return
  elements.errorTitle.textContent = localError ? localErrorTitle : '后端运行提示'
  elements.errorMessage.textContent = message
  elements.errorRetry.textContent = appState === null ? '重新加载' : '刷新状态'
  elements.errorRetry.disabled = stateLoading || busy !== null
}

function applyState(nextState: AppState): void {
  if (nextState.last_error !== appState?.last_error) dismissedBackendError = null
  appState = nextState
  initialLoading = false
  localError = null
  render()
}

async function loadState(silent = false): Promise<void> {
  if (stateLoading || busy !== null) return
  const requestId = ++stateRequestId
  stateLoading = true
  if (!silent && appState === null) initialLoading = true
  render()
  try {
    const nextState = await invoke<AppState>('get_state')
    if (requestId === stateRequestId) applyState(nextState)
  } catch (error) {
    if (requestId === stateRequestId) {
      initialLoading = false
      showError(formatError(error), '无法读取应用状态')
    }
  } finally {
    stateLoading = false
    if (requestId === stateRequestId) render()
  }
}

async function runCommand(
  operation: string,
  command: string,
  args?: CommandArgs,
  successMessage?: string,
): Promise<AppState | null> {
  if (busy !== null) return null
  busy = operation
  localError = null
  stateRequestId += 1
  stateLoading = false
  if (operation === 'add') setAddFormError(null)
  render()
  try {
    const nextState = args === undefined
      ? await invoke<AppState>(command)
      : await invoke<AppState>(command, args)
    applyState(nextState)
    if (successMessage) {
      showToast(nextState.last_error ? '操作未完全成功，请查看错误提示' : successMessage)
    }
    return nextState
  } catch (error) {
    const message = formatError(error)
    showError(message)
    if (operation === 'add') setAddFormError(message)
    return null
  } finally {
    busy = null
    render()
  }
}

function createSkeleton(count: number): DocumentFragment {
  const fragment = document.createDocumentFragment()
  const list = document.createElement('div')
  list.className = 'skeleton-list'
  list.setAttribute('aria-label', '正在加载')
  for (let index = 0; index < count; index += 1) {
    const row = document.createElement('div')
    row.className = 'skeleton-row'
    const circle = document.createElement('span')
    circle.className = 'skeleton-circle'
    const lines = document.createElement('div')
    lines.className = 'skeleton-lines'
    const line = document.createElement('span')
    line.className = 'skeleton-line'
    const shortLine = document.createElement('span')
    shortLine.className = 'skeleton-line short'
    lines.append(line, shortLine)
    row.append(circle, lines)
    list.append(row)
  }
  fragment.append(list)
  return fragment
}

function createEmptyState(
  title: string,
  description: string,
  options: { compact?: boolean; action?: string; actionLabel?: string; icon?: string } = {},
): HTMLDivElement {
  const empty = document.createElement('div')
  empty.className = `empty-state${options.compact ? ' compact' : ''}`
  const illustration = document.createElement('div')
  illustration.className = 'empty-illustration'
  illustration.append(createIcon(options.icon ?? 'i-nodes'))
  const heading = document.createElement('strong')
  heading.textContent = title
  const copy = document.createElement('p')
  copy.textContent = description
  empty.append(illustration, heading, copy)
  if (options.action && options.actionLabel) {
    const button = createButton(options.actionLabel, options.action, 'button secondary')
    button.disabled = !canInteract()
    empty.append(button)
  }
  return empty
}

function renderStatus(): void {
  elements.appStatus.className = 'status-pill'
  if (initialLoading) {
    elements.appStatus.classList.add('is-loading')
    elements.appStatusText.textContent = '连接中'
  } else if (appState === null) {
    elements.appStatus.classList.add('is-error')
    elements.appStatusText.textContent = '连接失败'
  } else if (appState.running) {
    elements.appStatus.classList.add('is-running')
    elements.appStatusText.textContent = '核心运行中'
  } else {
    elements.appStatusText.textContent = '核心未运行'
  }

  const subscriptions = appState?.subscriptions ?? []
  const nodeTotal = subscriptions.reduce((total, subscription) => total + subscription.nodes.length, 0)
  elements.subscriptionSummary.textContent = appState === null
    ? '正在读取配置'
    : `${subscriptions.length} 个订阅 · ${nodeTotal} 个节点`
  elements.subscriptionCount.textContent = `${subscriptions.length} 个`
}

function renderConnection(): void {
  const state = appState
  const subscription = currentSubscription()
  const nodes = currentNodes()
  const isProxyBusy = busy === 'proxy'

  elements.currentNodeName.textContent = initialLoading
    ? '正在读取…'
    : state?.current_node ?? (state?.auto_select ? '等待自动选择' : '未选择节点')
  elements.currentNodeName.title = state?.current_node ?? ''
  elements.currentSubscriptionName.textContent = subscription?.name ?? '未选择订阅'
  elements.currentSubscriptionName.title = subscription?.name ?? ''
  elements.currentNodeIndicator.classList.toggle('is-active', Boolean(state?.proxy_enabled || state?.running))
  elements.connectionBadge.classList.toggle('is-active', Boolean(state?.proxy_enabled || state?.running))
  elements.connectionBadge.textContent = state?.proxy_enabled
    ? '代理已开启'
    : state?.running ? '核心运行中' : '未连接'

  const canStart = subscription !== null && nodes.length > 0
  elements.proxyToggle.disabled = !canInteract() || (!state?.proxy_enabled && !canStart)
  elements.proxyToggle.classList.toggle('is-stop', Boolean(state?.proxy_enabled))
  elements.proxyToggle.classList.toggle('is-loading', isProxyBusy)
  elements.proxyToggleLabel.textContent = isProxyBusy
    ? state?.proxy_enabled ? '正在停止…' : '正在启动…'
    : state?.proxy_enabled ? '停止代理' : '启动代理'
  elements.proxyHint.textContent = !state?.proxy_enabled && subscription === null
    ? '请先添加并选择订阅'
    : !state?.proxy_enabled && nodes.length === 0
      ? '请先更新订阅以获取节点'
      : state?.tun_enabled
        ? 'TUN 全局模式已启用，系统流量由 EX1 接管'
        : state?.proxy_enabled
          ? 'Windows 系统代理已启用 · 127.0.0.1:10808'
          : '启动后将启用 Windows 系统代理'

  elements.autoSelectToggle.checked = state?.auto_select ?? false
  elements.autoSelectToggle.disabled = !canInteract()
  elements.autoStartToggle.checked = state?.auto_start ?? false
  elements.autoStartToggle.disabled = !canInteract()
  elements.silentStartToggle.checked = state?.silent_start ?? false
  elements.silentStartToggle.disabled = !canInteract()
  elements.tunToggle.checked = state?.tun_enabled ?? false
  elements.tunToggle.disabled = !canInteract()
}

function renderSubscriptions(): void {
  const list = elements.subscriptionList
  const scrollTop = list.scrollTop
  list.replaceChildren()
  if (initialLoading) {
    list.append(createSkeleton(3))
    return
  }
  if (appState === null) {
    list.append(createEmptyState('订阅信息暂不可用', '请检查应用状态后重试。', { compact: true, icon: 'i-alert' }))
    return
  }
  if (appState.subscriptions.length === 0) {
    list.append(createEmptyState('还没有订阅', '添加订阅后即可获取并管理节点。', {
      compact: true,
      action: 'add',
      actionLabel: '添加第一个订阅',
      icon: 'i-link',
    }))
    return
  }

  const fragment = document.createDocumentFragment()
  for (const subscription of appState.subscriptions) {
    const selected = subscription.id === appState.current_subscription
    const item = document.createElement('div')
    item.className = `subscription-item${selected ? ' is-selected' : ''}`
    item.setAttribute('role', 'listitem')

    const select = document.createElement('button')
    select.type = 'button'
    select.className = 'subscription-select'
    select.dataset.action = 'select-subscription'
    select.dataset.id = subscription.id
    select.disabled = !canInteract() || selected
    select.setAttribute('aria-pressed', String(selected))
    select.title = selected ? `当前订阅：${subscription.name}` : `选择订阅：${subscription.name}`
    const radio = document.createElement('span')
    radio.className = 'subscription-radio'
    const copy = document.createElement('span')
    copy.className = 'subscription-copy'
    const name = document.createElement('span')
    name.className = 'subscription-name'
    name.textContent = subscription.name
    name.title = subscription.name
    const meta = document.createElement('span')
    meta.className = 'subscription-meta'
    meta.textContent = `${subscription.nodes.length} 个节点`
    copy.append(name, meta)
    select.append(radio, copy)

    const actions = document.createElement('div')
    actions.className = 'subscription-actions'
    const update = createButton('', 'update-subscription')
    update.dataset.id = subscription.id
    update.disabled = !canInteract()
    update.classList.toggle('is-spinning', busy === `update:${subscription.id}`)
    update.setAttribute('aria-label', `更新订阅 ${subscription.name}`)
    update.title = '更新此订阅'
    update.append(createIcon('i-refresh'))
    const remove = createButton('', 'delete-subscription')
    remove.dataset.id = subscription.id
    remove.disabled = !canInteract()
    remove.setAttribute('aria-label', `删除订阅 ${subscription.name}`)
    remove.title = '删除订阅'
    remove.append(createIcon('i-trash'))
    actions.append(update, remove)
    item.append(select, actions)
    fragment.append(item)
  }
  list.append(fragment)
  list.scrollTop = scrollTop
}

function formatDelay(delay: number | null): { text: string; className: string } {
  if (delay === null || !Number.isFinite(delay)) return { text: '未测速', className: 'is-unknown' }
  if (delay <= 0) return { text: '超时', className: 'is-bad' }
  if (delay <= 180) return { text: `${Math.round(delay)} ms`, className: 'is-good' }
  if (delay <= 400) return { text: `${Math.round(delay)} ms`, className: 'is-medium' }
  return { text: `${Math.round(delay)} ms`, className: 'is-bad' }
}

function renderNodes(): void {
  const list = elements.nodeList
  const scrollTop = list.scrollTop
  const subscription = currentSubscription()
  const nodes = subscription?.nodes ?? []
  const autoSelect = appState?.auto_select ?? false
  elements.nodeCount.textContent = String(nodes.length)
  elements.nodesSubtitle.textContent = subscription
    ? `${subscription.name} · 点击节点即可切换出口`
    : '选择订阅后可查看可用节点'
  elements.nodesSubtitle.title = subscription?.name ?? ''
  elements.autoLockNote.hidden = !autoSelect

  list.replaceChildren()
  if (initialLoading) {
    list.append(createSkeleton(7))
    return
  }
  if (appState === null) {
    list.append(createEmptyState('节点信息暂不可用', '无法读取应用状态，请使用上方错误提示中的重试按钮。', { icon: 'i-alert' }))
    return
  }
  if (!subscription) {
    list.append(createEmptyState(
      appState.subscriptions.length === 0 ? '等待添加订阅' : '尚未选择订阅',
      appState.subscriptions.length === 0 ? '先在左侧添加订阅，节点会显示在这里。' : '从左侧订阅列表中选择一个订阅。',
      { icon: 'i-nodes' },
    ))
    return
  }
  if (nodes.length === 0) {
    list.append(createEmptyState('当前订阅没有节点', '更新订阅以获取最新节点，或检查订阅地址是否有效。', {
      action: 'update-current',
      actionLabel: '更新当前订阅',
      icon: 'i-refresh',
    }))
    return
  }

  const fragment = document.createDocumentFragment()
  for (const node of nodes) {
    const selected = node.name === appState.current_node
    const row = document.createElement('button')
    row.type = 'button'
    row.className = `node-row${selected ? ' is-selected' : ''}${busy === `node:${node.name}` ? ' is-busy' : ''}`
    row.dataset.action = 'select-node'
    row.dataset.name = node.name
    row.disabled = !canInteract() || autoSelect
    row.setAttribute('role', 'listitem')
    row.setAttribute('aria-pressed', String(selected))
    row.title = autoSelect ? '自动选择开启时无法手动选择节点' : `使用节点：${node.name}`

    const main = document.createElement('span')
    main.className = 'node-main'
    const dot = document.createElement('span')
    dot.className = 'node-state-dot'
    const name = document.createElement('span')
    name.className = 'node-name'
    name.textContent = node.name
    name.title = `${node.name} · ${node.server}:${node.port}`
    main.append(dot, name)

    const protocol = document.createElement('span')
    protocol.className = 'protocol-badge'
    protocol.textContent = node.protocol || '未知'
    protocol.title = node.protocol || '未知协议'

    const delay = formatDelay(node.delay)
    const delayElement = document.createElement('span')
    delayElement.className = `delay-value ${delay.className}`
    delayElement.textContent = delay.text

    const check = document.createElement('span')
    check.className = 'node-check'
    check.append(createIcon('i-check'))
    row.append(main, protocol, delayElement, check)
    fragment.append(row)
  }
  list.append(fragment)
  list.scrollTop = scrollTop
}

function renderActions(): void {
  const state = appState
  const subscription = currentSubscription()
  elements.addSubscription.disabled = !canInteract()
  elements.updateAll.disabled = !canInteract() || (state?.subscriptions.length ?? 0) === 0
  elements.updateAll.classList.toggle('is-loading', busy === 'update-all')
  elements.updateAllLabel.textContent = busy === 'update-all' ? '正在更新全部…' : '更新全部订阅'
  elements.refreshBest.disabled = !canInteract() || busy === 'refresh-best' || busy === 'auto-select' || !state?.auto_select || currentNodes().length === 0
  elements.refreshBest.classList.toggle('is-loading', busy === 'refresh-best')
  elements.refreshBestLabel.textContent = busy === 'refresh-best' ? '后台测速…' : '立即选优'
  elements.refreshBest.title = !state?.auto_select
    ? '开启自动选择后可立即选优'
    : subscription ? '后台并行测速，不影响当前连接' : '请先选择订阅'
}

function renderDialogState(): void {
  const addBusy = busy === 'add'
  elements.addName.disabled = busy !== null
  elements.addUrl.disabled = busy !== null
  elements.confirmAdd.disabled = busy !== null
  elements.confirmAdd.classList.toggle('is-loading', addBusy)
  elements.confirmAddLabel.textContent = addBusy ? '正在添加…' : '添加订阅'
  elements.confirmDelete.disabled = busy !== null
  elements.confirmDelete.classList.toggle('is-loading', busy === 'delete')
  elements.confirmDeleteLabel.textContent = busy === 'delete' ? '正在删除…' : '删除订阅'
  document.querySelectorAll<HTMLButtonElement>('[data-close-dialog]').forEach((button) => {
    button.disabled = busy !== null
  })
}

function render(): void {
  renderStatus()
  renderError()
  renderConnection()
  renderSubscriptions()
  renderNodes()
  renderActions()
  renderDialogState()
}

function setAddFormError(message: string | null): void {
  elements.addFormError.hidden = message === null
  elements.addFormError.textContent = message ?? ''
}

function openAddDialog(): void {
  if (!canInteract()) return
  elements.addForm.reset()
  setAddFormError(null)
  elements.addDialog.showModal()
  window.setTimeout(() => elements.addName.focus(), 0)
}

function openDeleteDialog(id: string): void {
  if (!canInteract() || !appState) return
  const subscription = appState.subscriptions.find((item) => item.id === id)
  if (!subscription) return
  pendingDeleteId = id
  elements.deleteSubscriptionName.textContent = subscription.name
  elements.deleteDialog.showModal()
}

function closeDialog(id: string): void {
  if (busy !== null) return
  const dialog = document.getElementById(id)
  if (dialog instanceof HTMLDialogElement) dialog.close()
}

elements.addSubscription.addEventListener('click', openAddDialog)

elements.updateAll.addEventListener('click', () => {
  void runCommand('update-all', 'update_all_subscriptions', undefined, '全部订阅已更新')
})

elements.refreshBest.addEventListener('click', () => {
  if (!appState?.auto_select) return
  void runCommand('refresh-best', 'refresh_best_node', undefined, '已重新测速并选择最佳节点')
})

elements.proxyToggle.addEventListener('click', () => {
  if (!appState) return
  const enabled = !appState.proxy_enabled
  void runCommand('proxy', 'set_proxy_enabled', { enabled }, enabled ? '代理已启动' : '代理已停止')
})

elements.autoSelectToggle.addEventListener('change', () => {
  const enabled = elements.autoSelectToggle.checked
  void runCommand('auto-select', 'set_auto_select', { enabled }, enabled ? '自动选择已开启' : '自动选择已关闭，可手动选择节点')
})

elements.autoStartToggle.addEventListener('change', () => {
  const enabled = elements.autoStartToggle.checked
  void runCommand('auto-start', 'set_auto_start', { enabled }, enabled ? '已开启隐藏开机自启动' : '已关闭开机自启动')
})

elements.silentStartToggle.addEventListener('change', () => {
  const enabled = elements.silentStartToggle.checked
  void runCommand('silent-start', 'set_silent_start', { enabled }, enabled ? '登录后将静默启动代理，不显示主窗口' : '已关闭静默启动')
})

elements.tunToggle.addEventListener('change', () => {
  const enabled = elements.tunToggle.checked
  void runCommand('tun', 'set_tun_enabled', { enabled }, enabled ? 'TUN 全局模式已开启' : 'TUN 全局模式已关闭，已恢复原代理')
})

elements.subscriptionList.addEventListener('click', (event) => {
  const target = event.target
  if (!(target instanceof Element)) return
  const button = target.closest<HTMLButtonElement>('button[data-action]')
  if (!button || button.disabled || busy !== null) return
  const action = button.dataset.action
  const id = button.dataset.id
  if (action === 'add') {
    openAddDialog()
    return
  }
  if (!id || !appState) return
  const subscription = appState.subscriptions.find((item) => item.id === id)
  if (!subscription) return
  if (action === 'select-subscription') {
    void runCommand(`subscription:${id}`, 'select_subscription', { id }, `已选择订阅「${subscription.name}」`)
  } else if (action === 'update-subscription') {
    void runCommand(`update:${id}`, 'update_subscription', { id }, `订阅「${subscription.name}」已更新`)
  } else if (action === 'delete-subscription') {
    openDeleteDialog(id)
  }
})

elements.nodeList.addEventListener('click', (event) => {
  const target = event.target
  if (!(target instanceof Element)) return
  const button = target.closest<HTMLButtonElement>('button[data-action]')
  if (!button || button.disabled || busy !== null || !appState) return
  if (button.dataset.action === 'update-current') {
    const subscription = currentSubscription()
    if (subscription) {
      void runCommand(`update:${subscription.id}`, 'update_subscription', { id: subscription.id }, `订阅「${subscription.name}」已更新`)
    }
    return
  }
  if (button.dataset.action !== 'select-node' || appState.auto_select) return
  const name = button.dataset.name
  if (!name || name === appState.current_node) return
  void runCommand(`node:${name}`, 'select_node', { name }, `已切换到节点「${name}」`)
})

elements.addForm.addEventListener('submit', (event) => {
  event.preventDefault()
  if (busy !== null) return
  const name = elements.addName.value.trim()
  const url = elements.addUrl.value.trim()
  if (!name) {
    setAddFormError('请输入订阅名称。')
    elements.addName.focus()
    return
  }
  if (!url) {
    setAddFormError('请输入订阅地址。')
    elements.addUrl.focus()
    return
  }
  try {
    const parsedUrl = new URL(url)
    if (parsedUrl.protocol !== 'http:' && parsedUrl.protocol !== 'https:') throw new Error('protocol')
  } catch {
    setAddFormError('订阅地址无效，请输入完整的 http:// 或 https:// 地址。')
    elements.addUrl.focus()
    return
  }
  void runCommand('add', 'add_subscription', { name, url }, `订阅「${name}」已添加`).then((result) => {
    if (result) elements.addDialog.close()
  })
})

elements.confirmDelete.addEventListener('click', () => {
  if (!pendingDeleteId || busy !== null || !appState) return
  const id = pendingDeleteId
  const subscription = appState.subscriptions.find((item) => item.id === id)
  if (!subscription) return
  void runCommand('delete', 'delete_subscription', { id }, `订阅「${subscription.name}」已删除`).then((result) => {
    if (result) {
      pendingDeleteId = null
      elements.deleteDialog.close()
    }
  })
})

elements.errorRetry.addEventListener('click', () => {
  void loadState()
})

elements.errorClose.addEventListener('click', () => {
  if (localError) {
    localError = null
  } else if (appState?.last_error) {
    dismissedBackendError = appState.last_error
  }
  renderError()
})

document.querySelectorAll<HTMLButtonElement>('[data-close-dialog]').forEach((button) => {
  button.addEventListener('click', () => closeDialog(button.dataset.closeDialog ?? ''))
})

for (const dialog of [elements.addDialog, elements.deleteDialog]) {
  dialog.addEventListener('cancel', (event) => {
    if (busy !== null) event.preventDefault()
  })
}

for (const input of [elements.addName, elements.addUrl]) {
  input.addEventListener('input', () => setAddFormError(null))
}

window.addEventListener('focus', () => {
  if (appState !== null) void loadState(true)
})

document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible' && appState !== null) void loadState(true)
})

window.setInterval(() => {
  if (document.visibilityState === 'visible' && appState !== null) void loadState(true)
}, 15000)

async function startApp(): Promise<void> {
  try {
    await listen<AppState>('ex1-state', (event) => {
      if (busy === null) applyState(event.payload)
    })
  } catch (error) {
    showError(formatError(error), '无法订阅后台状态')
  }
  render()
  await loadState()
}

void startApp()

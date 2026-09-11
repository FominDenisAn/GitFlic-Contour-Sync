<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import type {
  BranchItem,
  ContourConfig,
  HistoryEntry,
  Locale,
  ProjectItem,
  RunnerResult,
  SyncRequest,
  SyncState,
  ThemeMode,
} from './lib/types'
import { messages, type MessageKey } from './lib/i18n'
import { authenticateContour, listBranches, listProjects, normalizeContourUrl, runSyncPipeline } from './lib/tauri'

type AppTab = 'overview' | 'compare' | 'branches' | 'changes' | 'execution' | 'history'
type NavKey = 'projects' | 'contours' | 'sync' | 'history' | 'settings'
type UiError = { title: string; message: string; context?: string; occurredAt: string }

const TRANSFER_OWNER = 'devops'
const TRANSFER_PROJECT = 'images-transfer'
const TRANSFER_BRANCH = 'repositories'
const HISTORY_KEY = 'gcs-history-v1'

const theme = ref<ThemeMode>('dark')
const locale = ref<Locale>('ru')
const activeNav = ref<NavKey>('sync')
const activeTab = ref<AppTab>('overview')
const runnerTag = ref('')
const projectSearchLeft = ref('Pipeline')
const projectSearchRight = ref('Pipeline')
const selectedLeft = ref<ProjectItem | null>(null)
const selectedRight = ref<ProjectItem | null>(null)
const leftProjects = ref<ProjectItem[]>([])
const rightProjects = ref<ProjectItem[]>([])
const leftBranches = ref<BranchItem[]>([])
const rightBranches = ref<BranchItem[]>([])
const sourceBranch = ref('')
const targetBranch = ref('')
const uiError = ref<UiError | null>(null)
const infoMessage = ref('')
const loadingLeft = ref(false)
const loadingRight = ref(false)
const branchLoadingLeft = ref(false)
const branchLoadingRight = ref(false)
const runnerBusy = ref(false)
const runnerAction = ref<'preview' | 'push' | null>(null)
const previewResult = ref<RunnerResult | null>(null)
const lastRun = ref<RunnerResult | null>(null)
const history = ref<HistoryEntry[]>([])
const compareResult = ref<null | { source: BranchItem; target: BranchItem; sameHead: boolean }>(null)

const left = ref<ContourConfig>({ label: 'Contour 1', url: '', token: '', connected: false, user: null })
const right = ref<ContourConfig>({ label: 'Contour 2', url: '', token: '', connected: false, user: null })

function t(key: MessageKey): string {
  return messages[locale.value][key]
}

const tabs = computed<Array<{ key: AppTab; label: string }>>(() => [
  { key: 'overview', label: t('tabOverview') },
  { key: 'compare', label: t('tabCompare') },
  { key: 'branches', label: t('tabBranches') },
  { key: 'changes', label: t('tabChanges') },
  { key: 'execution', label: t('tabExecution') },
  { key: 'history', label: t('tabHistory') },
])

const navItems = computed<Array<{ key: NavKey; label: string }>>(() => [
  { key: 'projects', label: t('navProjects') },
  { key: 'contours', label: t('navContours') },
  { key: 'sync', label: t('navSync') },
  { key: 'history', label: t('navHistory') },
  { key: 'settings', label: t('navSettings') },
])

const readyToCompare = computed(() => Boolean(
  left.value.connected && right.value.connected && selectedLeft.value && selectedRight.value
  && sourceBranch.value && targetBranch.value && runnerTag.value.trim() && !runnerBusy.value,
))

const canPush = computed(() => Boolean(
  previewResult.value
  && previewResult.value.action === 'preview'
  && previewResult.value.state === 'DEV_AHEAD'
  && previewResult.value.preflightPassed
  && previewResult.value.dryRunPassed
  && !runnerBusy.value,
))

const sidebarUser = computed(() => left.value.user)
const sidebarName = computed(() => sidebarUser.value?.fullName?.trim() || sidebarUser.value?.username || t('userFallback'))
const sidebarAlias = computed(() => sidebarUser.value?.username ? `@${sidebarUser.value.username}` : t('contourFallback'))
const sidebarInitials = computed(() => {
  const parts = sidebarName.value.trim().split(/\s+/).filter(Boolean)
  return parts.slice(0, 2).map((part) => part[0]?.toUpperCase()).join('') || 'GF'
})

function applyTheme(mode: ThemeMode) {
  localStorage.setItem('gcs-theme', mode)
  const resolved = mode === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : mode
  document.documentElement.dataset.theme = resolved
}

function applyLocale(value: Locale) {
  const changed = locale.value !== value
  locale.value = value
  localStorage.setItem('gcs-locale', value)
  document.documentElement.lang = value
  if (changed) {
    uiError.value = null
    infoMessage.value = ''
  }
}

function loadHistory() {
  try {
    const parsed = JSON.parse(localStorage.getItem(HISTORY_KEY) || '[]')
    history.value = Array.isArray(parsed) ? parsed.slice(0, 100) : []
  } catch {
    history.value = []
  }
}

function saveHistory() {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value.slice(0, 100)))
}

onMounted(() => {
  theme.value = (localStorage.getItem('gcs-theme') as ThemeMode | null) ?? 'dark'
  locale.value = (localStorage.getItem('gcs-locale') as Locale | null) ?? 'ru'
  applyTheme(theme.value)
  applyLocale(locale.value)
  loadHistory()
})

watch(theme, applyTheme)
watch([sourceBranch, targetBranch], () => resetSyncState())

function clearMessages() {
  uiError.value = null
  infoMessage.value = ''
}

function resetSyncState() {
  compareResult.value = null
  previewResult.value = null
}

function showError(title: string, error: unknown, context?: string) {
  uiError.value = {
    title,
    message: error instanceof Error ? error.message : String(error),
    context,
    occurredAt: new Date().toLocaleString(locale.value === 'ru' ? 'ru-RU' : 'en-US'),
  }
}

async function copyError() {
  if (!uiError.value) return
  const text = [uiError.value.title, uiError.value.context, uiError.value.message, uiError.value.occurredAt].filter(Boolean).join('\n')
  try {
    await navigator.clipboard.writeText(text)
    infoMessage.value = t('copied')
  } catch {
    infoMessage.value = t('copyFailed')
  }
}

function navClick(key: NavKey) {
  activeNav.value = key
  if (key === 'history') activeTab.value = 'history'
  else if (key === 'sync' || key === 'projects' || key === 'contours') activeTab.value = 'overview'
}

async function connect(side: 'left' | 'right', contour: ContourConfig) {
  clearMessages()
  const loading = side === 'left' ? loadingLeft : loadingRight
  const contourName = side === 'left' ? t('contour1') : t('contour2')
  loading.value = true
  try {
    contour.url = await normalizeContourUrl(contour.url, locale.value)
    if (!contour.token.trim()) throw new Error(t('tokenRequired'))
    const auth = await authenticateContour(contour.url, contour.token, locale.value)
    contour.url = auth.baseUrl
    contour.user = auth.user
    contour.connected = true
    await refreshProjects(side)
    infoMessage.value = `${contourName}: ${t('signedInAs')} - ${auth.user.fullName?.trim() || auth.user.username}`
  } catch (error) {
    contour.connected = false
    contour.user = null
    if (side === 'left') {
      leftProjects.value = []; selectedLeft.value = null; leftBranches.value = []; sourceBranch.value = ''
    } else {
      rightProjects.value = []; selectedRight.value = null; rightBranches.value = []; targetBranch.value = ''
    }
    showError(`${contourName}: ${t('authError')}`, error, `${t('url')}: ${contour.url}`)
  } finally {
    loading.value = false
  }
}

async function refreshProjects(side: 'left' | 'right') {
  const contour = side === 'left' ? left.value : right.value
  if (!contour.connected) return
  const query = side === 'left' ? projectSearchLeft.value : projectSearchRight.value
  const loading = side === 'left' ? loadingLeft : loadingRight
  const contourName = side === 'left' ? t('contour1') : t('contour2')
  loading.value = true
  try {
    const items = await listProjects(contour.url, contour.token, query, locale.value)
    if (side === 'left') leftProjects.value = items
    else rightProjects.value = items
  } catch (error) {
    showError(`${contourName}: ${t('projectsError')}`, error, `${t('searchContext')}: ${query || t('noFilter')}`)
  } finally {
    loading.value = false
  }
}

let leftSearchTimer: ReturnType<typeof setTimeout> | null = null
let rightSearchTimer: ReturnType<typeof setTimeout> | null = null
watch(projectSearchLeft, () => {
  if (!left.value.connected) return
  if (leftSearchTimer) clearTimeout(leftSearchTimer)
  leftSearchTimer = setTimeout(() => refreshProjects('left'), 350)
})
watch(projectSearchRight, () => {
  if (!right.value.connected) return
  if (rightSearchTimer) clearTimeout(rightSearchTimer)
  rightSearchTimer = setTimeout(() => refreshProjects('right'), 350)
})

async function selectProject(side: 'left' | 'right', project: ProjectItem) {
  clearMessages(); resetSyncState()
  const contour = side === 'left' ? left.value : right.value
  const loading = side === 'left' ? branchLoadingLeft : branchLoadingRight
  loading.value = true
  try {
    const items = await listBranches(contour.url, contour.token, project.owner, project.alias, locale.value)
    if (side === 'left') {
      selectedLeft.value = project; leftBranches.value = items; sourceBranch.value = pickInitialBranch(project, items)
    } else {
      selectedRight.value = project; rightBranches.value = items; targetBranch.value = pickInitialBranch(project, items)
    }
  } catch (error) {
    showError(t('branchesError'), error, `${project.owner}/${project.alias}`)
  } finally {
    loading.value = false
  }
}

function pickInitialBranch(project: ProjectItem, items: BranchItem[]) {
  if (!items.length) return ''
  const preferred = project.defaultBranch || project.workBranch
  return items.find((item) => item.name === preferred)?.name || items.find((item) => item.default)?.name || items[0].name
}

function selectedBranch(items: BranchItem[], name: string) {
  return items.find((item) => item.name === name) || null
}

function requestId() {
  return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function buildSyncRequest(action: 'preview' | 'push'): SyncRequest {
  const source = selectedBranch(leftBranches.value, sourceBranch.value)
  const target = selectedBranch(rightBranches.value, targetBranch.value)
  if (!selectedLeft.value || !selectedRight.value || !source || !target) throw new Error(t('chooseProjectsFirst'))
  if (!runnerTag.value.trim()) throw new Error(t('runnerRequired'))

  const expectedSource = action === 'push'
    ? (previewResult.value?.actualSourceSha || previewResult.value?.sourceSha || source.sha)
    : source.sha
  const expectedTarget = action === 'push'
    ? (previewResult.value?.actualTargetSha || previewResult.value?.targetSha || target.sha)
    : target.sha

  return {
    controlUrl: left.value.url,
    controlToken: left.value.token,
    controlOwner: TRANSFER_OWNER,
    controlProject: TRANSFER_PROJECT,
    controlBranch: TRANSFER_BRANCH,
    runnerTag: runnerTag.value.trim(),
    sourceUrl: left.value.url,
    sourceOwner: selectedLeft.value.owner,
    sourceProject: selectedLeft.value.alias,
    sourceBranch: sourceBranch.value,
    sourceSha: expectedSource,
    targetUrl: right.value.url,
    targetOwner: selectedRight.value.owner,
    targetProject: selectedRight.value.alias,
    targetBranch: targetBranch.value,
    targetSha: expectedTarget,
    action,
    requestId: requestId(),
    locale: locale.value,
  }
}

function addHistory(result: RunnerResult, request: SyncRequest) {
  history.value.unshift({
    id: result.requestId || request.requestId,
    createdAt: new Date().toISOString(),
    action: request.action,
    sourceProject: `${request.sourceOwner}/${request.sourceProject}`,
    sourceBranch: request.sourceBranch,
    targetProject: `${request.targetOwner}/${request.targetProject}`,
    targetBranch: request.targetBranch,
    sourceSha: result.actualSourceSha || request.sourceSha,
    targetSha: result.actualTargetSha || request.targetSha,
    resultSha: result.postTargetSha,
    state: result.state,
    pipelineLocalId: result.pipelineLocalId,
    changedFiles: result.changedFiles,
    additions: result.additions,
    deletions: result.deletions,
  })
  saveHistory()
}

async function compareChanges() {
  clearMessages()
  const source = selectedBranch(leftBranches.value, sourceBranch.value)
  const target = selectedBranch(rightBranches.value, targetBranch.value)
  if (!source || !target) return
  compareResult.value = { source, target, sameHead: source.sha === target.sha }

  if (source.sha === target.sha) {
    previewResult.value = null
    activeTab.value = 'compare'
    return
  }

  let request: SyncRequest
  try {
    request = buildSyncRequest('preview')
  } catch (error) {
    showError(t('comparisonError'), error)
    return
  }

  runnerBusy.value = true
  runnerAction.value = 'preview'
  activeTab.value = 'compare'
  try {
    const result = await runSyncPipeline(request)
    previewResult.value = result
    lastRun.value = result
    addHistory(result, request)
    if (result.state === 'DEV_AHEAD' && result.preflightPassed && result.dryRunPassed) activeTab.value = 'changes'
  } catch (error) {
    showError(t('comparisonError'), error, `${selectedLeft.value?.owner}/${selectedLeft.value?.alias} · ${sourceBranch.value}`)
  } finally {
    runnerBusy.value = false
    runnerAction.value = null
  }
}

async function updateBranch() {
  if (!canPush.value) return
  clearMessages()
  let request: SyncRequest
  try {
    request = buildSyncRequest('push')
  } catch (error) {
    showError(t('updateError'), error)
    return
  }

  runnerBusy.value = true
  runnerAction.value = 'push'
  activeTab.value = 'execution'
  try {
    const result = await runSyncPipeline(request)
    lastRun.value = result
    addHistory(result, request)
    if (result.state === 'SYNCED') {
      infoMessage.value = t('updateCompleted')
      await reloadBranches()
      previewResult.value = result
      const source = selectedBranch(leftBranches.value, sourceBranch.value)
      const target = selectedBranch(rightBranches.value, targetBranch.value)
      if (source && target) compareResult.value = { source, target, sameHead: source.sha === target.sha }
    } else {
      previewResult.value = result
      infoMessage.value = `${stateLabel(result.state)}. ${t('updateBlocked')}`
      activeTab.value = 'compare'
    }
  } catch (error) {
    showError(t('updateError'), error, `${selectedRight.value?.owner}/${selectedRight.value?.alias} · ${targetBranch.value}`)
  } finally {
    runnerBusy.value = false
    runnerAction.value = null
  }
}

async function reloadBranches() {
  if (selectedLeft.value) {
    leftBranches.value = await listBranches(left.value.url, left.value.token, selectedLeft.value.owner, selectedLeft.value.alias, locale.value)
  }
  if (selectedRight.value) {
    rightBranches.value = await listBranches(right.value.url, right.value.token, selectedRight.value.owner, selectedRight.value.alias, locale.value)
  }
}

function clearHistory() {
  history.value = []
  saveHistory()
}

function formatDate(value?: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat(locale.value === 'ru' ? 'ru-RU' : 'en-US', {
    day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit',
  }).format(date)
}

function shortSha(value?: string | null) {
  return value ? value.slice(0, 8) : '—'
}

function stateLabel(state?: SyncState | string) {
  switch (state) {
    case 'SYNCED': return t('synchronized')
    case 'DEV_AHEAD': return t('sourceAhead')
    case 'PROD_AHEAD': return t('targetAhead')
    case 'DIVERGED': return t('diverged')
    case 'SOURCE_CHANGED': return t('sourceChanged')
    case 'TARGET_CHANGED': return t('targetChanged')
    case 'ERROR': return t('statusFailed')
    default: return t('unknownState')
  }
}

function stateMessage(state?: SyncState | string) {
  switch (state) {
    case 'SYNCED': return t('noUpdateRequired')
    case 'DEV_AHEAD': return previewResult.value?.dryRunPassed ? t('safeToUpdate') : t('updateBlocked')
    case 'PROD_AHEAD': return t('targetNewerBlocked')
    case 'DIVERGED': return t('divergedBlocked')
    case 'SOURCE_CHANGED':
    case 'TARGET_CHANGED': return t('stateChangedBlocked')
    case 'ERROR': return previewResult.value?.message || t('statusFailed')
    default: return previewResult.value?.message || t('updateBlocked')
  }
}

function stateClass(state?: SyncState | string) {
  if (state === 'SYNCED') return 'synced'
  if (state === 'DEV_AHEAD') return 'ahead'
  return 'blocked'
}

function pipelineStatus(value?: string) {
  switch ((value || '').toUpperCase()) {
    case 'SUCCESS': return t('statusSuccess')
    case 'FAILED': return t('statusFailed')
    case 'RUNNING': return t('statusRunning')
    case 'CREATED': case 'PENDING': return t('statusWaiting')
    case 'CANCELED': case 'CANCELLED': return t('statusCancelled')
    case 'SKIPPED': return t('statusSkipped')
    case 'WARNING': return t('statusWarning')
    default: return value || t('statusWaiting')
  }
}

function statusClass(value?: string) {
  const normalized = (value || '').toUpperCase()
  if (normalized === 'SUCCESS') return 'ok'
  if (normalized === 'FAILED' || normalized === 'CANCELED' || normalized === 'CANCELLED') return 'error'
  if (normalized === 'RUNNING') return 'running'
  return 'waiting'
}

function fileStatus(status: string) {
  if (status === 'A') return t('fileAdded')
  if (status === 'D') return t('fileDeleted')
  if (status === 'R') return t('fileRenamed')
  return t('fileModified')
}

function historyAction(action: 'preview' | 'push') {
  return action === 'push' ? t('push') : t('preview')
}
</script>

<template>
  <div class="app-shell">
    <header class="topbar">
      <div class="brand"><span class="brand-mark">◆</span><strong>GitFlic</strong></div>
      <div class="topbar-spacer" />
      <button class="topbar-link">⚒ {{ t('adminPanel') }}</button>
      <div class="global-search"><span>{{ t('search') }}</span><span class="search-icon">⌕</span></div>
      <div class="theme-switch" :aria-label="t('theme')">
        <button :title="t('lightTheme')" :class="{ active: theme === 'light' }" @click="theme = 'light'">☀</button>
        <button :title="t('systemTheme')" :class="{ active: theme === 'system' }" @click="theme = 'system'">◉</button>
        <button :title="t('darkTheme')" :class="{ active: theme === 'dark' }" @click="theme = 'dark'">☾</button>
      </div>
      <button :class="['locale', { active: locale === 'ru' }]" @click="applyLocale('ru')">RU</button>
      <button :class="['locale', { active: locale === 'en' }]" @click="applyLocale('en')">ENG</button>
      <span class="avatar">{{ sidebarInitials }}</span>
    </header>

    <aside class="sidebar">
      <div class="user-card">
        <div class="avatar big">{{ sidebarInitials }}</div>
        <div><strong>{{ sidebarName }}</strong><small>{{ sidebarAlias }}</small></div>
      </div>
      <nav>
        <button v-for="item in navItems" :key="item.key" :class="{ active: activeNav === item.key }" @click="navClick(item.key)">
          <span class="nav-icon">▣</span>{{ item.label }}
        </button>
      </nav>
      <div class="sidebar-bottom">
        <button><span class="nav-icon">▣</span>{{ t('news') }}</button>
        <button><span class="nav-icon">◉</span>{{ t('help') }}</button>
      </div>
    </aside>

    <main class="content">
      <div class="page-head"><div class="breadcrumb"><span>devops</span><span>/</span><strong>{{ t('appName') }}</strong></div></div>
      <div class="tabs">
        <button v-for="tab in tabs" :key="tab.key" :class="{ active: activeTab === tab.key }" @click="activeTab = tab.key">{{ tab.label }}</button>
      </div>

      <section v-if="uiError" class="error-panel">
        <div class="error-panel-head">
          <div><strong>{{ uiError.title }}</strong><small>{{ uiError.occurredAt }}</small></div>
          <button class="secondary compact" @click="copyError">{{ t('copy') }}</button>
        </div>
        <div v-if="uiError.context" class="error-context">{{ uiError.context }}</div>
        <pre>{{ uiError.message }}</pre>
      </section>
      <div v-if="infoMessage" class="info-banner">{{ infoMessage }}</div>

      <template v-if="activeTab === 'overview'">
        <section class="connection-grid">
          <div class="connection-column">
            <h2>{{ t('contour1') }}</h2>
            <label>{{ t('url') }}</label>
            <input v-model="left.url" placeholder="https://git.example.local" />
            <label>{{ t('apiToken') }}</label>
            <input v-model="left.token" type="password" autocomplete="off" :placeholder="t('tokenPlaceholder')" />
            <div class="panel-actions">
              <button class="secondary" :disabled="loadingLeft" @click="connect('left', left)">{{ loadingLeft ? t('signingIn') : t('signIn') }}</button>
              <span v-if="left.connected" class="status ok"><i />{{ left.user?.fullName || left.user?.username }}</span>
              <span v-else class="status"><i />{{ t('notSignedIn') }}</span>
            </div>
          </div>
          <div class="flow-arrow">→</div>
          <div class="connection-column">
            <h2>{{ t('contour2') }}</h2>
            <label>{{ t('url') }}</label>
            <input v-model="right.url" placeholder="https://git.example.local" />
            <label>{{ t('apiToken') }}</label>
            <input v-model="right.token" type="password" autocomplete="off" :placeholder="t('tokenPlaceholder')" />
            <div class="panel-actions">
              <button class="secondary" :disabled="loadingRight" @click="connect('right', right)">{{ loadingRight ? t('signingIn') : t('signIn') }}</button>
              <span v-if="right.connected" class="status ok"><i />{{ right.user?.fullName || right.user?.username }}</span>
              <span v-else class="status"><i />{{ t('notSignedIn') }}</span>
            </div>
          </div>
        </section>

        <section class="runner-row">
          <strong>{{ t('runner') }}</strong>
          <label>{{ t('runnerTag') }}</label>
          <input v-model.trim="runnerTag" list="runner-tag-suggestions" :placeholder="t('runnerPlaceholder')" />
          <datalist id="runner-tag-suggestions"><option value="registry" /><option value="repository-transfer" /><option value="prod-sk" /></datalist>
        </section>

        <section class="project-grid">
          <div class="project-column">
            <div class="column-head"><h3>{{ t('projects') }}</h3><input v-model="projectSearchLeft" :placeholder="t('projectSearch')" /></div>
            <div v-if="left.connected && loadingLeft" class="empty-state">{{ t('loadingProjects') }}</div>
            <div v-else-if="left.connected && !leftProjects.length" class="empty-state">{{ t('noProjects') }}</div>
            <div v-else-if="!left.connected" class="empty-state">{{ t('signInContour1') }}</div>
            <div v-else class="project-list">
              <button v-for="project in leftProjects" :key="project.id" :class="['project-card', { selected: selectedLeft?.id === project.id }]" @click="selectProject('left', project)">
                <strong>{{ project.owner }}/<span>{{ project.name }}</span></strong><small>{{ project.description || project.defaultBranch || t('projectFallback') }}</small>
              </button>
            </div>
          </div>
          <div class="project-column">
            <div class="column-head"><h3>{{ t('projects') }}</h3><input v-model="projectSearchRight" :placeholder="t('projectSearch')" /></div>
            <div v-if="right.connected && loadingRight" class="empty-state">{{ t('loadingProjects') }}</div>
            <div v-else-if="right.connected && !rightProjects.length" class="empty-state">{{ t('noProjects') }}</div>
            <div v-else-if="!right.connected" class="empty-state">{{ t('signInContour2') }}</div>
            <div v-else class="project-list">
              <button v-for="project in rightProjects" :key="project.id" :class="['project-card', { selected: selectedRight?.id === project.id }]" @click="selectProject('right', project)">
                <strong>{{ project.owner }}/<span>{{ project.name }}</span></strong><small>{{ project.description || project.defaultBranch || t('projectFallback') }}</small>
              </button>
            </div>
          </div>
        </section>

        <section v-if="selectedLeft && selectedRight" class="branch-panel">
          <div class="branch-side">
            <strong>{{ selectedLeft.owner }} / {{ selectedLeft.name }}</strong><label>{{ t('branch') }}</label>
            <select v-model="sourceBranch" :disabled="branchLoadingLeft || !leftBranches.length">
              <option v-if="!leftBranches.length" value="">{{ t('noBranches') }}</option>
              <option v-for="b in leftBranches" :key="b.name" :value="b.name">{{ b.name }}</option>
            </select>
          </div>
          <div class="branch-arrow">→</div>
          <div class="branch-side">
            <strong>{{ selectedRight.owner }} / {{ selectedRight.name }}</strong><label>{{ t('branch') }}</label>
            <select v-model="targetBranch" :disabled="branchLoadingRight || !rightBranches.length">
              <option v-if="!rightBranches.length" value="">{{ t('noBranches') }}</option>
              <option v-for="b in rightBranches" :key="b.name" :value="b.name">{{ b.name }}</option>
            </select>
          </div>
          <button class="primary compare-btn" :disabled="!readyToCompare" @click="compareChanges">{{ runnerBusy && runnerAction === 'preview' ? t('comparingChanges') : t('compareChanges') }}</button>
        </section>
      </template>

      <section v-else-if="activeTab === 'compare'" class="page-panel compare-result">
        <template v-if="compareResult">
          <div class="compare-title">
            <h2>{{ t('compareTitle') }}</h2>
            <span :class="['compare-state', previewResult ? stateClass(previewResult.state) : { synced: compareResult.sameHead }]">
              {{ compareResult.sameHead ? t('synchronized') : (previewResult ? stateLabel(previewResult.state) : t('different')) }}
            </span>
          </div>
          <div class="route-line">{{ selectedLeft?.owner }}/{{ selectedLeft?.alias }} · {{ sourceBranch }} <span>→</span> {{ selectedRight?.owner }}/{{ selectedRight?.alias }} · {{ targetBranch }}</div>
          <div class="head-grid">
            <div><small>{{ t('contour1') }}</small><span class="commit-label">{{ t('lastCommit') }}</span><strong>{{ compareResult.source.sha }}</strong><span>{{ formatDate(compareResult.source.updatedAt) }}</span></div>
            <div><small>{{ t('contour2') }}</small><span class="commit-label">{{ t('lastCommit') }}</span><strong>{{ compareResult.target.sha }}</strong><span>{{ formatDate(compareResult.target.updatedAt) }}</span></div>
          </div>
          <div v-if="runnerBusy" class="runner-progress"><span class="spinner" />{{ runnerAction === 'push' ? t('updatingBranch') : t('comparingChanges') }}</div>
          <div v-else-if="compareResult.sameHead" class="result-ok">{{ t('noUpdateRequired') }}</div>
          <div v-else-if="previewResult" :class="previewResult.state === 'DEV_AHEAD' && previewResult.dryRunPassed ? 'result-ok' : 'result-note'">{{ stateMessage(previewResult.state) }}</div>
          <div v-else class="result-note">{{ t('different') }}</div>
          <div v-if="previewResult" class="summary-strip">
            <span><strong>{{ previewResult.changedFiles }}</strong>{{ t('changedFiles') }}</span>
            <span class="plus"><strong>+{{ previewResult.additions }}</strong>{{ t('additions') }}</span>
            <span class="minus"><strong>-{{ previewResult.deletions }}</strong>{{ t('deletions') }}</span>
          </div>
        </template>
        <div v-else class="empty-page">{{ t('chooseProjectsFirst') }}</div>
      </section>

      <section v-else-if="activeTab === 'branches'" class="page-panel">
        <div class="section-heading"><h2>{{ t('branchesTitle') }}</h2></div>
        <div class="branches-grid">
          <div class="branch-list-panel"><h3>{{ t('contour1') }}</h3>
            <div v-for="branch in leftBranches" :key="branch.name" class="branch-row"><div><strong>{{ branch.name }}</strong><small>{{ formatDate(branch.updatedAt) }}</small></div><code>{{ shortSha(branch.sha) }}</code></div>
            <div v-if="!leftBranches.length" class="empty-state flat">{{ selectedLeft ? t('emptyRepository') : t('noData') }}</div>
          </div>
          <div class="branch-list-panel"><h3>{{ t('contour2') }}</h3>
            <div v-for="branch in rightBranches" :key="branch.name" class="branch-row"><div><strong>{{ branch.name }}</strong><small>{{ formatDate(branch.updatedAt) }}</small></div><code>{{ shortSha(branch.sha) }}</code></div>
            <div v-if="!rightBranches.length" class="empty-state flat">{{ selectedRight ? t('emptyRepository') : t('noData') }}</div>
          </div>
        </div>
      </section>

      <section v-else-if="activeTab === 'changes'" class="page-panel changes-panel">
        <template v-if="previewResult">
          <div class="section-heading">
            <h2>{{ t('changesTitle') }}</h2>
            <span :class="['compare-state', stateClass(previewResult.state)]">{{ stateLabel(previewResult.state) }}</span>
          </div>
          <div class="route-line">{{ selectedLeft?.owner }}/{{ selectedLeft?.alias }} · {{ sourceBranch }} <span>→</span> {{ selectedRight?.owner }}/{{ selectedRight?.alias }} · {{ targetBranch }}</div>
          <div class="summary-strip wide">
            <span><strong>{{ previewResult.changedFiles }}</strong>{{ t('changedFiles') }}</span>
            <span class="plus"><strong>+{{ previewResult.additions }}</strong>{{ t('additions') }}</span>
            <span class="minus"><strong>-{{ previewResult.deletions }}</strong>{{ t('deletions') }}</span>
            <span><strong>{{ previewResult.commits.length }}</strong>{{ t('commits') }}</span>
          </div>
          <div v-if="previewResult.truncated" class="result-note">{{ t('diffTruncated') }}</div>
          <div class="diff-files">
            <details v-for="file in previewResult.files" :key="`${file.status}-${file.path}`" class="diff-file" open>
              <summary>
                <span class="file-state">{{ file.status }}</span>
                <strong>{{ file.oldPath ? `${file.oldPath} → ${file.path}` : file.path }}</strong>
                <small>{{ fileStatus(file.status) }}</small>
                <span class="diff-count plus">+{{ file.additions }}</span><span class="diff-count minus">-{{ file.deletions }}</span>
              </summary>
              <div v-if="file.binary" class="binary-row">{{ t('binaryFile') }}</div>
              <div v-else class="diff-table">
                <div v-for="(line, index) in file.lines" :key="index" :class="['diff-line', line.kind]">
                  <span class="line-no">{{ line.oldNo ?? '' }}</span><span class="line-no">{{ line.newNo ?? '' }}</span><code>{{ line.kind === 'add' ? '+' : line.kind === 'remove' ? '-' : ' ' }}{{ line.text }}</code>
                </div>
              </div>
            </details>
            <div v-if="!previewResult.files.length" class="empty-page">{{ t('noData') }}</div>
          </div>
          <div class="write-actions">
            <button class="secondary" :disabled="runnerBusy" @click="compareChanges">{{ t('refreshComparison') }}</button>
            <button class="primary" :disabled="!canPush" @click="updateBranch">{{ runnerBusy && runnerAction === 'push' ? t('updatingBranch') : t('updateBranch') }}</button>
          </div>
        </template>
        <div v-else class="empty-page">{{ t('changesEmpty') }}</div>
      </section>

      <section v-else-if="activeTab === 'execution'" class="page-panel">
        <div class="section-heading"><h2>{{ t('executionTitle') }}</h2></div>
        <div v-if="runnerBusy" class="runner-progress large"><span class="spinner" />{{ runnerAction === 'push' ? t('updatingBranch') : t('comparingChanges') }}</div>
        <template v-else-if="lastRun">
          <div class="execution-meta">
            <span>{{ t('pipeline') }} <strong>#{{ lastRun.pipelineLocalId }}</strong></span>
            <span :class="['job-status', statusClass(lastRun.pipelineStatus)]">{{ pipelineStatus(lastRun.pipelineStatus) }}</span>
            <span>{{ stateLabel(lastRun.state) }}</span>
          </div>
          <div v-if="lastRun.message" class="runner-message">{{ lastRun.message }}</div>
          <div class="job-list">
            <div v-for="job in lastRun.jobs" :key="job.localId" class="job-row">
              <div><strong>{{ job.name }}</strong><small v-if="job.stageName">{{ t('stage') }}: {{ job.stageName }}</small></div>
              <span :class="['job-status', statusClass(job.status)]">{{ pipelineStatus(job.status) }}</span>
            </div>
          </div>
        </template>
        <div v-else class="empty-page">{{ t('executionEmpty') }}</div>
      </section>

      <section v-else class="page-panel history-panel">
        <div class="section-heading"><h2>{{ t('historyTitle') }}</h2><button v-if="history.length" class="secondary compact" @click="clearHistory">{{ t('clearHistory') }}</button></div>
        <div v-if="history.length" class="history-list">
          <div v-for="entry in history" :key="entry.id" class="history-row">
            <div class="history-main"><strong>{{ historyAction(entry.action) }}</strong><span>{{ entry.sourceProject }} · {{ entry.sourceBranch }} → {{ entry.targetProject }} · {{ entry.targetBranch }}</span><small>{{ formatDate(entry.createdAt) }}</small></div>
            <div class="history-stats"><span>{{ stateLabel(entry.state) }}</span><code>#{{ entry.pipelineLocalId }}</code><small>+{{ entry.additions }} / -{{ entry.deletions }}</small></div>
          </div>
        </div>
        <div v-else class="empty-page">{{ t('historyEmpty') }}</div>
      </section>
    </main>
  </div>
</template>

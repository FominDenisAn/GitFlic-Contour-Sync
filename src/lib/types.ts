export type ThemeMode = 'light' | 'system' | 'dark'
export type Locale = 'ru' | 'en'

export interface UserProfile {
  id: string
  username: string
  name?: string | null
  surname?: string | null
  fullName?: string | null
  email?: string | null
}

export interface ContourConfig {
  label: string
  url: string
  token: string
  connected: boolean
  user?: UserProfile | null
}

export interface AuthResult {
  baseUrl: string
  user: UserProfile
}

export interface ProjectItem {
  id: string
  owner: string
  ownerType?: string | null
  name: string
  alias: string
  description?: string | null
  defaultBranch?: string | null
  workBranch?: string | null
  private: boolean
}

export interface BranchItem {
  name: string
  sha: string
  updatedAt?: string | null
  default: boolean
  work: boolean
}

export type SyncState =
  | 'SYNCED'
  | 'DEV_AHEAD'
  | 'PROD_AHEAD'
  | 'DIVERGED'
  | 'SOURCE_CHANGED'
  | 'TARGET_CHANGED'
  | 'ERROR'
  | 'UNKNOWN'

export interface DiffLine {
  oldNo?: number | null
  newNo?: number | null
  kind: 'context' | 'add' | 'remove'
  text: string
}

export interface DiffFile {
  path: string
  oldPath?: string | null
  status: 'M' | 'A' | 'D' | 'R' | string
  additions: number
  deletions: number
  binary: boolean
  lines: DiffLine[]
}

export interface PipelineJob {
  name: string
  status: string
  localId: number
  stageName?: string | null
}

export interface RunnerResult {
  requestId: string
  pipelineUuid: string
  pipelineLocalId: number
  pipelineStatus: string
  action: 'preview' | 'push' | string
  state: SyncState
  sourceSha: string
  targetSha?: string | null
  actualSourceSha?: string | null
  actualTargetSha?: string | null
  postTargetSha?: string | null
  preflightPassed: boolean
  dryRunPassed: boolean
  changedFiles: number
  additions: number
  deletions: number
  message: string
  files: DiffFile[]
  commits: string[]
  jobs: PipelineJob[]
  truncated: boolean
}

export interface SyncRequest {
  controlUrl: string
  controlToken: string
  controlOwner: string
  controlProject: string
  controlBranch: string
  runnerTag: string
  sourceUrl: string
  sourceOwner: string
  sourceProject: string
  sourceBranch: string
  sourceSha: string
  targetUrl: string
  targetOwner: string
  targetProject: string
  targetBranch: string
  targetSha?: string | null
  action: 'preview' | 'push'
  requestId: string
  locale: Locale
}

export interface HistoryEntry {
  id: string
  createdAt: string
  action: 'preview' | 'push'
  sourceProject: string
  sourceBranch: string
  targetProject: string
  targetBranch: string
  sourceSha: string
  targetSha?: string | null
  resultSha?: string | null
  state: SyncState
  pipelineLocalId: number
  changedFiles: number
  additions: number
  deletions: number
}

import { invoke } from '@tauri-apps/api/core'
import type { AuthResult, BranchItem, Locale, ProjectItem, RunnerResult, SyncRequest } from './types'

export function runningInTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function desktopOnly(locale: Locale): never {
  throw new Error(locale === 'ru'
    ? 'Эта операция доступна только в приложении.'
    : 'This operation is available only in the desktop application.')
}

export async function authenticateContour(url: string, token: string, locale: Locale): Promise<AuthResult> {
  if (!runningInTauri()) desktopOnly(locale)
  return invoke<AuthResult>('authenticate_contour', { url, token, locale })
}

export async function listProjects(url: string, token: string, query: string, locale: Locale): Promise<ProjectItem[]> {
  if (!runningInTauri()) desktopOnly(locale)
  return invoke<ProjectItem[]>('list_projects', { url, token, query, locale })
}

export async function listBranches(url: string, token: string, owner: string, project: string, locale: Locale): Promise<BranchItem[]> {
  if (!runningInTauri()) desktopOnly(locale)
  return invoke<BranchItem[]>('list_branches', { url, token, owner, project, locale })
}

export async function normalizeContourUrl(url: string, locale: Locale): Promise<string> {
  if (!runningInTauri()) {
    const parsed = new URL(url.includes('://') ? url : `https://${url}`)
    return `${parsed.protocol}//${parsed.host}`
  }
  return invoke<string>('normalize_contour_url', { url, locale })
}

export async function runSyncPipeline(request: SyncRequest): Promise<RunnerResult> {
  if (!runningInTauri()) desktopOnly(request.locale)
  return invoke<RunnerResult>('run_sync_pipeline', { request })
}

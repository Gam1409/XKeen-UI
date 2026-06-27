import { apiCall } from '@/lib/api'
import type { Subscription, SubscriptionFormValues, SubscriptionPreview, SubscriptionUpdateResult } from '@/types/subscription'

interface ApiResult<T = unknown> {
  success: boolean
  error?: string
  items?: T[]
  item?: T
  lines?: string[]
  total?: number
  warnings?: string[]
}

type UpdateApiResult = ApiResult & Partial<SubscriptionUpdateResult>

function ensure<T>(result: ApiResult<T>): ApiResult<T> {
  if (!result.success) throw new Error(result.error || 'Запрос не выполнен')
  return result
}

export async function listSubscriptions(): Promise<Subscription[]> {
  const result = ensure(await apiCall<ApiResult<Subscription>>('GET', 'subscriptions'))
  return result.items || []
}

export async function createSubscription(values: SubscriptionFormValues): Promise<Subscription> {
  const result = ensure(await apiCall<ApiResult<Subscription>>('POST', 'subscriptions', values))
  if (!result.item) throw new Error('Пустой ответ сервера')
  return result.item
}

export async function updateSubscription(id: string, values: SubscriptionFormValues): Promise<Subscription> {
  const result = ensure(await apiCall<ApiResult<Subscription>>('PUT', `subscriptions/${id}`, values))
  if (!result.item) throw new Error('Пустой ответ сервера')
  return result.item
}

export async function deleteSubscription(id: string, deleteOutputFile: boolean): Promise<void> {
  ensure(await apiCall<ApiResult>('DELETE', `subscriptions/${id}`, { deleteOutputFile, removeCron: true, backupBeforeDelete: true }))
}

export async function checkSubscription(id: string): Promise<SubscriptionUpdateResult> {
  const result = ensure(await apiCall<UpdateApiResult>('POST', `subscriptions/${id}/check`, {}))
  return updateResult(result)
}

export async function runSubscriptionUpdate(id: string, noRestart = false): Promise<SubscriptionUpdateResult> {
  const result = ensure(
    await apiCall<UpdateApiResult>('POST', `subscriptions/${id}/update`, {
      dryRun: false,
      noRestart,
      withBackup: true,
    })
  )
  return updateResult(result)
}

export async function rollbackSubscriptionUpdate(id: string): Promise<SubscriptionUpdateResult> {
  const result = ensure(await apiCall<UpdateApiResult>('POST', `subscriptions/${id}/rollback`, {}))
  return updateResult(result)
}

export async function previewSubscription(id: string): Promise<SubscriptionPreview> {
  const result = ensure(await apiCall<ApiResult<SubscriptionPreview['items'][number]>>('GET', `subscriptions/${id}/preview`))
  return {
    items: result.items || [],
    total: result.total || 0,
    warnings: result.warnings || [],
  }
}

export async function setSubscriptionEnabled(item: Subscription, enabled: boolean): Promise<Subscription> {
  return updateSubscription(item.id, {
    name: item.name,
    enabled,
    url: undefined,
    core: item.core,
    mode: item.mode,
    outputTag: item.outputTag,
    outputDir: item.outputDir,
    autoRestart: item.autoRestart,
    singleProxy: item.singleProxy,
    realityFingerprint: item.realityFingerprint,
    dialerProxies: item.dialerProxies,
    updateInterval: item.updateInterval,
    timeoutSec: item.timeoutSec,
    allowInsecureUrl: item.allowInsecureUrl,
    providerName: item.providerName,
    providerPath: item.providerPath,
    providerGroup: item.providerGroup,
    providerGroupType: item.providerGroupType,
    providerHealthCheck: item.providerHealthCheck,
    providerHealthCheckUrl: item.providerHealthCheckUrl,
    providerHealthCheckInterval: item.providerHealthCheckInterval,
    nativeInclude: item.nativeInclude,
    nativeExclude: item.nativeExclude,
  })
}

export async function getSubscriptionLog(id: string): Promise<string[]> {
  const result = ensure(await apiCall<ApiResult>('GET', `subscriptions/${id}/log?tail=200`))
  return result.lines || []
}

export async function setSubscriptionSchedule(id: string, updateInterval: string): Promise<void> {
  ensure(await apiCall<ApiResult>('POST', `subscriptions/${id}/schedule`, { updateInterval }))
}

export async function deleteSubscriptionSchedule(id: string): Promise<void> {
  ensure(await apiCall<ApiResult>('DELETE', `subscriptions/${id}/schedule`, {}))
}

function updateResult(result: UpdateApiResult): SubscriptionUpdateResult {
  if (!result.id || !result.status || typeof result.nodeCount !== 'number' || !result.outputFile || typeof result.dryRun !== 'boolean') {
    throw new Error('Пустой ответ сервера')
  }
  return {
    id: result.id,
    status: result.status,
    nodeCount: result.nodeCount,
    outputFile: result.outputFile,
    dryRun: result.dryRun,
    restarted: !!result.restarted,
    message: result.message || '',
  }
}

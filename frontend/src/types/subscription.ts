export type SubscriptionStatus = 'never' | 'ok' | 'warning' | 'error'

export interface Subscription {
  id: string
  name: string
  enabled: boolean
  urlMasked: string
  core: 'xray' | 'mihomo'
  mode: 'watcher' | 'provider' | 'native'
  format: string
  outputTag: string
  outputDir: string
  outputFile: string
  autoRestart: boolean
  singleProxy: boolean
  realityFingerprint: string
  dialerProxies: string[]
  updateInterval: string
  timeoutSec: number
  allowInsecureUrl: boolean
  providerName: string
  providerPath: string
  providerGroup: string
  providerGroupType: 'select' | 'url-test' | 'fallback' | 'load-balance'
  providerHealthCheck: boolean
  providerHealthCheckUrl: string
  providerHealthCheckInterval: number
  nativeInclude: string
  nativeExclude: string
  lastUpdateAt: string | null
  lastSuccessAt: string | null
  lastStatus: SubscriptionStatus
  lastError: string
  lastNodeCount: number
  lastHash: string
  createdAt: string
  updatedAt: string
}

export interface SubscriptionFormValues {
  id?: string
  name: string
  url?: string
  enabled?: boolean
  core?: 'xray' | 'mihomo'
  mode?: 'watcher' | 'provider' | 'native'
  format?: string
  outputTag?: string
  outputDir?: string
  autoRestart?: boolean
  singleProxy?: boolean
  realityFingerprint?: string
  dialerProxies?: string[]
  updateInterval?: string
  timeoutSec?: number
  allowInsecureUrl?: boolean
  providerName?: string
  providerPath?: string
  providerGroup?: string
  providerGroupType?: 'select' | 'url-test' | 'fallback' | 'load-balance'
  providerHealthCheck?: boolean
  providerHealthCheckUrl?: string
  providerHealthCheckInterval?: number
  nativeInclude?: string
  nativeExclude?: string
}

export interface SubscriptionUpdateResult {
  id: string
  status: string
  nodeCount: number
  outputFile: string
  dryRun: boolean
  restarted: boolean
  message: string
}

export interface SubscriptionPreviewNode {
  name: string
  protocol: string
  server: string
  port: number
  warning: string
}

export interface SubscriptionPreview {
  items: SubscriptionPreviewNode[]
  total: number
  warnings: string[]
}

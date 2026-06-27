import {
  checkSubscription,
  createSubscription,
  deleteSubscription,
  deleteSubscriptionSchedule,
  getSubscriptionLog,
  listSubscriptions,
  previewSubscription,
  rollbackSubscriptionUpdate,
  runSubscriptionUpdate,
  setSubscriptionEnabled,
  setSubscriptionSchedule,
  updateSubscription,
} from '@/api/subscriptions'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Textarea } from '@/components/ui/textarea'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { showToast } from '@/lib/store'
import { cn } from '@/lib/utils'
import type { Subscription, SubscriptionFormValues, SubscriptionPreview, SubscriptionStatus } from '@/types/subscription'
import {
  IconCheck,
  IconClock,
  IconDotsVertical,
  IconEdit,
  IconFileText,
  IconListDetails,
  IconPlayerPlay,
  IconPlus,
  IconRefresh,
  IconRestore,
  IconTrash,
  IconX,
} from '@tabler/icons-react'
import { useCallback, useEffect, useMemo, useState } from 'react'

const DEFAULT_FORM: SubscriptionFormValues = {
  name: '',
  url: '',
  enabled: true,
  core: 'xray',
  mode: 'watcher',
  format: 'auto',
  outputTag: '',
  outputDir: '/opt/etc/xray/configs',
  autoRestart: true,
  singleProxy: false,
  realityFingerprint: '',
  dialerProxies: [],
  updateInterval: '0 */6 * * *',
  timeoutSec: 20,
  allowInsecureUrl: false,
  providerName: '',
  providerPath: './providers/sub-main.yaml',
  providerGroup: '',
  providerGroupType: 'select',
  providerHealthCheck: true,
  providerHealthCheckUrl: 'https://www.gstatic.com/generate_204',
  providerHealthCheckInterval: 300,
  nativeInclude: '',
  nativeExclude: '',
}

function statusVariant(status: SubscriptionStatus): 'emerald' | 'amber' | 'rose' | 'secondary' {
  if (status === 'ok') return 'emerald'
  if (status === 'warning') return 'amber'
  if (status === 'error') return 'rose'
  return 'secondary'
}

function statusText(status: SubscriptionStatus) {
  if (status === 'ok') return 'OK'
  if (status === 'warning') return 'Предупреждение'
  if (status === 'error') return 'Ошибка'
  return 'Не обновлялась'
}

function formatDate(value: string | null) {
  if (!value) return '—'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat('ru-RU', {
    day: '2-digit',
    month: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}

function formFromSubscription(item: Subscription): SubscriptionFormValues {
  return {
    name: item.name,
    url: '',
    enabled: item.enabled,
    core: item.core,
    mode: item.mode,
    format: item.format,
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
  }
}

function serializeForm(values: SubscriptionFormValues, editing?: Subscription | null): SubscriptionFormValues {
  return {
    ...values,
    id: editing ? undefined : values.id?.trim() || undefined,
    url: values.url?.trim() ? values.url.trim() : editing ? undefined : '',
    name: values.name.trim(),
    outputTag: values.outputTag?.trim(),
    outputDir: values.outputDir?.trim(),
    realityFingerprint: values.realityFingerprint?.trim(),
    updateInterval: values.updateInterval?.trim(),
    timeoutSec: Number(values.timeoutSec || 20),
    dialerProxies: values.dialerProxies?.map((v) => v.trim()).filter(Boolean),
    providerName: values.providerName?.trim(),
    providerPath: values.providerPath?.trim(),
    providerGroup: values.providerGroup?.trim(),
    providerGroupType: values.providerGroupType,
    providerHealthCheck: values.providerHealthCheck,
    providerHealthCheckUrl: values.providerHealthCheckUrl?.trim(),
    providerHealthCheckInterval: Number(values.providerHealthCheckInterval || 300),
    nativeInclude: values.nativeInclude?.trim(),
    nativeExclude: values.nativeExclude?.trim(),
  }
}

interface SubscriptionDialogProps {
  item: Subscription | null
  open: boolean
  onOpenChange: (open: boolean) => void
  onSaved: () => Promise<void>
}

function SubscriptionDialog({ item, open, onOpenChange, onSaved }: SubscriptionDialogProps) {
  const [values, setValues] = useState<SubscriptionFormValues>(DEFAULT_FORM)
  const [dialerText, setDialerText] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (!open) return
    const next = item ? formFromSubscription(item) : DEFAULT_FORM
    setValues(next)
    setDialerText((next.dialerProxies || []).join('\n'))
  }, [item, open])

  const setField = <K extends keyof SubscriptionFormValues>(key: K, value: SubscriptionFormValues[K]) =>
    setValues((prev) => ({ ...prev, [key]: value }))

  const isMihomo = values.core === 'mihomo'

  function setMode(value: 'xray-watcher' | 'xray-native' | 'mihomo-provider') {
    const core = value === 'mihomo-provider' ? 'mihomo' : 'xray'
    const mode = value === 'xray-native' ? 'native' : value === 'mihomo-provider' ? 'provider' : 'watcher'
    setValues((prev) => ({
      ...prev,
      core,
      mode,
      outputDir: core === 'mihomo' ? '/opt/etc/mihomo' : prev.outputDir || '/opt/etc/xray/configs',
      providerName: prev.providerName || prev.outputTag || prev.id || 'sub-main',
    }))
  }

  async function save() {
    setSaving(true)
    try {
      const body = serializeForm({ ...values, dialerProxies: dialerText.split(/\r?\n/) }, item)
      if (item) await updateSubscription(item.id, body)
      else await createSubscription(body)
      showToast(item ? 'Подписка обновлена' : 'Подписка создана')
      await onSaved()
      onOpenChange(false)
    } catch (e: any) {
      showToast(e.message, 'error')
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{item ? 'Изменить подписку' : 'Добавить подписку'}</DialogTitle>
          <DialogDescription>{item ? item.urlMasked : 'Xray / подписки'}</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 sm:grid-cols-2">
          {!item && (
            <div className="grid gap-2">
              <Label htmlFor="sub-id">ID</Label>
              <Input id="sub-id" value={values.id || ''} onChange={(e) => setField('id', e.target.value)} placeholder="main-vps" />
            </div>
          )}
          <div className="grid gap-2">
            <Label htmlFor="sub-name">Название</Label>
            <Input id="sub-name" value={values.name} onChange={(e) => setField('name', e.target.value)} placeholder="Main VPS" />
          </div>
          <div className="grid gap-2">
            <Label>Ядро / режим</Label>
            <Select
              value={values.core === 'mihomo' ? 'mihomo-provider' : values.mode === 'native' ? 'xray-native' : 'xray-watcher'}
              onValueChange={(value) => setMode(value as 'xray-watcher' | 'xray-native' | 'mihomo-provider')}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="xray-watcher">Xray через watcher</SelectItem>
                  <SelectItem value="xray-native">Xray native</SelectItem>
                  <SelectItem value="mihomo-provider">Mihomo provider</SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-2 sm:col-span-2">
            <Label htmlFor="sub-url">URL</Label>
            <Input
              id="sub-url"
              value={values.url || ''}
              onChange={(e) => setField('url', e.target.value)}
              placeholder={item ? 'Оставьте пустым, чтобы не менять URL' : 'https://example.com/sub/token'}
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="sub-tag">Тег output</Label>
            <Input id="sub-tag" value={values.outputTag || ''} onChange={(e) => setField('outputTag', e.target.value)} placeholder="sub-main" />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="sub-dir">Папка output</Label>
            <Input id="sub-dir" value={values.outputDir || ''} onChange={(e) => setField('outputDir', e.target.value)} />
          </div>
        </div>

        <div className="grid gap-3 rounded-lg border p-3 sm:grid-cols-3">
          <label className="flex items-center justify-between gap-3 text-sm">
            Включена
            <Switch checked={!!values.enabled} onCheckedChange={(checked) => setField('enabled', checked)} />
          </label>
          <label className="flex items-center justify-between gap-3 text-sm">
            Автоперезапуск
            <Switch checked={!!values.autoRestart} onCheckedChange={(checked) => setField('autoRestart', checked)} />
          </label>
          <label className="flex items-center justify-between gap-3 text-sm">
            Один proxy
            <Switch checked={!!values.singleProxy} onCheckedChange={(checked) => setField('singleProxy', checked)} />
          </label>
        </div>

        <details className="rounded-lg border p-3">
          <summary className="cursor-pointer text-sm font-medium">Дополнительно</summary>
          <div className="mt-3 grid gap-4 sm:grid-cols-2">
            <div className="grid gap-2">
              <Label htmlFor="sub-cron">Cron</Label>
              <Input id="sub-cron" value={values.updateInterval || ''} onChange={(e) => setField('updateInterval', e.target.value)} />
            </div>
            <div className="grid gap-2">
            <Label htmlFor="sub-timeout">Таймаут, сек</Label>
              <Input
                id="sub-timeout"
                type="number"
                min={1}
                max={300}
                value={values.timeoutSec || 20}
                onChange={(e) => setField('timeoutSec', Number(e.target.value))}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="sub-fp">Reality fingerprint</Label>
              <Input id="sub-fp" value={values.realityFingerprint || ''} onChange={(e) => setField('realityFingerprint', e.target.value)} />
            </div>
            <label className="flex items-end justify-between gap-3 pb-2 text-sm">
              Разрешить HTTP
              <Switch checked={!!values.allowInsecureUrl} onCheckedChange={(checked) => setField('allowInsecureUrl', checked)} />
            </label>
            {isMihomo && (
              <>
                <div className="grid gap-2">
                  <Label htmlFor="sub-provider-name">Имя provider</Label>
                  <Input
                    id="sub-provider-name"
                    value={values.providerName || ''}
                    onChange={(e) => setField('providerName', e.target.value)}
                    placeholder="sub-main"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="sub-provider-path">Путь provider</Label>
                  <Input
                    id="sub-provider-path"
                    value={values.providerPath || ''}
                    onChange={(e) => setField('providerPath', e.target.value)}
                    placeholder="./providers/sub-main.yaml"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="sub-provider-group">Группа proxy</Label>
                  <Input
                    id="sub-provider-group"
                    value={values.providerGroup || ''}
                    onChange={(e) => setField('providerGroup', e.target.value)}
                    placeholder="Auto"
                  />
                </div>
                <div className="grid gap-2">
                  <Label>Тип группы</Label>
                  <Select
                    value={values.providerGroupType || 'select'}
                    onValueChange={(value) => setField('providerGroupType', value as SubscriptionFormValues['providerGroupType'])}
                  >
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectItem value="select">select</SelectItem>
                        <SelectItem value="url-test">url-test</SelectItem>
                        <SelectItem value="fallback">fallback</SelectItem>
                        <SelectItem value="load-balance">load-balance</SelectItem>
                      </SelectGroup>
                    </SelectContent>
                  </Select>
                </div>
                <label className="flex items-end justify-between gap-3 pb-2 text-sm">
                  Проверка доступности
                  <Switch checked={!!values.providerHealthCheck} onCheckedChange={(checked) => setField('providerHealthCheck', checked)} />
                </label>
                <div className="grid gap-2">
                  <Label htmlFor="sub-provider-hc-interval">Интервал проверки</Label>
                  <Input
                    id="sub-provider-hc-interval"
                    type="number"
                    min={30}
                    max={86400}
                    value={values.providerHealthCheckInterval || 300}
                    onChange={(e) => setField('providerHealthCheckInterval', Number(e.target.value))}
                  />
                </div>
                <div className="grid gap-2 sm:col-span-2">
                  <Label htmlFor="sub-provider-hc-url">URL проверки</Label>
                  <Input
                    id="sub-provider-hc-url"
                    value={values.providerHealthCheckUrl || ''}
                    onChange={(e) => setField('providerHealthCheckUrl', e.target.value)}
                  />
                </div>
              </>
            )}
            {values.mode === 'native' && (
              <>
                <div className="grid gap-2">
                  <Label htmlFor="sub-native-include">Включить</Label>
                  <Input
                    id="sub-native-include"
                    value={values.nativeInclude || ''}
                    onChange={(e) => setField('nativeInclude', e.target.value)}
                    placeholder="jp"
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="sub-native-exclude">Исключить</Label>
                  <Input
                    id="sub-native-exclude"
                    value={values.nativeExclude || ''}
                    onChange={(e) => setField('nativeExclude', e.target.value)}
                    placeholder="test"
                  />
                </div>
              </>
            )}
            <div className="grid gap-2 sm:col-span-2">
              <Label htmlFor="sub-dialer">Dialer proxies</Label>
              <Textarea id="sub-dialer" value={dialerText} onChange={(e) => setDialerText(e.target.value)} />
            </div>
          </div>
        </details>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Отмена
          </Button>
          <Button onClick={save} disabled={saving || !values.name.trim() || (!item && !values.url?.trim())}>
            {saving ? 'Сохранение...' : 'Сохранить'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

interface LogDialogProps {
  item: Subscription | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

function LogDialog({ item, open, onOpenChange }: LogDialogProps) {
  const [lines, setLines] = useState<string[]>([])

  useEffect(() => {
    if (!open || !item) return
    getSubscriptionLog(item.id)
      .then(setLines)
      .catch((e: any) => showToast(e.message, 'error'))
  }, [item, open])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>Лог подписки</DialogTitle>
          <DialogDescription>{item?.name}</DialogDescription>
        </DialogHeader>
        <pre className="bg-muted min-h-64 overflow-auto rounded-lg p-3 font-mono text-xs whitespace-pre-wrap">
          {lines.length ? lines.join('\n') : 'Лог пуст'}
        </pre>
      </DialogContent>
    </Dialog>
  )
}

interface PreviewDialogProps {
  item: Subscription | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

function PreviewDialog({ item, open, onOpenChange }: PreviewDialogProps) {
  const [preview, setPreview] = useState<SubscriptionPreview | null>(null)

  useEffect(() => {
    if (!open || !item) return
    setPreview(null)
    previewSubscription(item.id)
      .then(setPreview)
      .catch((e: any) => showToast(e.message, 'error'))
  }, [item, open])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>Предпросмотр</DialogTitle>
          <DialogDescription>{item?.name}</DialogDescription>
        </DialogHeader>
        {preview?.warnings.length ? (
          <div className="border-border bg-muted/40 rounded-md border p-2 text-xs">
            {preview.warnings.slice(0, 10).map((warning, index) => (
              <div key={index} className="truncate">
                {warning}
              </div>
            ))}
          </div>
        ) : null}
        <div className="overflow-hidden rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Название</TableHead>
                <TableHead>Протокол</TableHead>
                <TableHead>Сервер</TableHead>
                <TableHead>Порт</TableHead>
                <TableHead>Предупреждение</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {preview ? (
                preview.items.length ? (
                  preview.items.map((node, index) => (
                    <TableRow key={`${node.name}-${index}`}>
                      <TableCell className="max-w-64 truncate">{node.name}</TableCell>
                      <TableCell>{node.protocol}</TableCell>
                      <TableCell className="max-w-64 truncate font-mono text-xs">{node.server}</TableCell>
                      <TableCell>{node.port || ''}</TableCell>
                      <TableCell className="max-w-72 truncate text-xs">{node.warning}</TableCell>
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={5} className="text-muted-foreground h-20 text-center">
                      Узлы не найдены
                    </TableCell>
                  </TableRow>
                )
              ) : (
                <TableRow>
                  <TableCell colSpan={5} className="text-muted-foreground h-20 text-center">
                    Загрузка...
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
        {preview && <div className="text-muted-foreground text-xs">Показано {preview.items.length} из {preview.total}</div>}
      </DialogContent>
    </Dialog>
  )
}

export default function SubscriptionsPanel() {
  const [items, setItems] = useState<Subscription[]>([])
  const [loading, setLoading] = useState(true)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [editing, setEditing] = useState<Subscription | null>(null)
  const [formOpen, setFormOpen] = useState(false)
  const [logItem, setLogItem] = useState<Subscription | null>(null)
  const [previewItem, setPreviewItem] = useState<Subscription | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setItems(await listSubscriptions())
    } catch (e: any) {
      showToast(e.message, 'error')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const hasItems = items.length > 0

  const commandById = useMemo(
    () => Object.fromEntries(items.map((item) => [item.id, `xkeen-ui subscription-update ${item.id}`])),
    [items]
  )

  async function runAction(id: string, action: () => Promise<unknown>, success: string) {
    setBusyId(id)
    try {
      await action()
      showToast(success)
      await load()
    } catch (e: any) {
      showToast(e.message, 'error')
    } finally {
      setBusyId(null)
    }
  }

  return (
    <TooltipProvider delayDuration={500}>
      <div className="border-border bg-card flex flex-col gap-3 rounded-xl border p-3 sm:p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <h2 className="text-lg font-semibold">Подписки</h2>
            <p className="text-muted-foreground text-xs">Xray и Mihomo</p>
          </div>
          <div className="flex gap-1.5">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="outline" size="icon" onClick={load} disabled={loading}>
                  <IconRefresh />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Обновить список</TooltipContent>
            </Tooltip>
            <Button
              onClick={() => {
                setEditing(null)
                setFormOpen(true)
              }}
            >
              <IconPlus data-icon="inline-start" /> Добавить
            </Button>
          </div>
        </div>

        <div className="overflow-hidden rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-12">On</TableHead>
                <TableHead>Название</TableHead>
                <TableHead>URL</TableHead>
                <TableHead>Узлов</TableHead>
                <TableHead>Статус</TableHead>
                <TableHead>Обновлено</TableHead>
                <TableHead>Output</TableHead>
                <TableHead className="w-12" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={8} className="text-muted-foreground h-24 text-center">
                    Загрузка...
                  </TableCell>
                </TableRow>
              ) : hasItems ? (
                items.map((item) => (
                  <TableRow key={item.id}>
                    <TableCell>
                      <Switch
                        size="sm"
                        checked={item.enabled}
                        disabled={busyId === item.id}
                        onCheckedChange={(enabled) => runAction(item.id, () => setSubscriptionEnabled(item, enabled), 'Статус изменён')}
                      />
                    </TableCell>
                    <TableCell>
                      <div className="max-w-48 truncate font-medium">{item.name}</div>
                      <div className="text-muted-foreground flex max-w-48 items-center gap-1 truncate text-xs">
                        <span>{item.core}/{item.mode}</span>
                        <span>{item.core === 'mihomo' ? item.providerName : item.outputTag}</span>
                      </div>
                    </TableCell>
                    <TableCell className="max-w-64 truncate font-mono text-xs">{item.urlMasked}</TableCell>
                    <TableCell>{item.lastNodeCount}</TableCell>
                    <TableCell>
                      <Badge variant={statusVariant(item.lastStatus)}>{statusText(item.lastStatus)}</Badge>
                      {item.lastError && <div className="text-muted-foreground mt-1 max-w-48 truncate text-xs">{item.lastError}</div>}
                    </TableCell>
                    <TableCell>{formatDate(item.lastUpdateAt)}</TableCell>
                    <TableCell className="max-w-72 truncate font-mono text-xs">{item.outputFile}</TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon" disabled={busyId === item.id}>
                            <IconDotsVertical />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="min-w-56">
                          {item.mode === 'native' && (
                            <DropdownMenuItem onClick={() => setPreviewItem(item)}>
                              <IconListDetails /> Предпросмотр
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuItem onClick={() => runAction(item.id, () => checkSubscription(item.id), 'Проверка выполнена')}>
                            <IconCheck /> Проверить
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => runAction(item.id, () => runSubscriptionUpdate(item.id), 'Обновление выполнено')}>
                            <IconPlayerPlay /> Обновить сейчас
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => runAction(item.id, () => rollbackSubscriptionUpdate(item.id), 'Восстановлено')}>
                            <IconRestore /> Восстановить
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() => {
                              setEditing(item)
                              setFormOpen(true)
                            }}
                          >
                            <IconEdit /> Изменить
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => setLogItem(item)}>
                            <IconFileText /> Лог
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={() =>
                              navigator.clipboard
                                .writeText(commandById[item.id])
                                .then(() => showToast('Команда скопирована'))
                                .catch((e: any) => showToast(e.message, 'error'))
                            }
                          >
                            <IconClock /> Скопировать команду
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => runAction(item.id, () => setSubscriptionSchedule(item.id, item.updateInterval), 'Cron обновлён')}>
                            <IconClock /> Включить cron
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={() => runAction(item.id, () => deleteSubscriptionSchedule(item.id), 'Cron удалён')}>
                            <IconX /> Отключить cron
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            variant="destructive"
                            onClick={() => {
                              if (!confirm(`Удалить подписку «${item.name}»?`)) return
                              runAction(item.id, () => deleteSubscription(item.id, false), 'Подписка удалена')
                            }}
                          >
                            <IconTrash /> Удалить
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={8} className="text-muted-foreground h-24 text-center">
                    Подписок нет
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>

        <div className={cn('text-muted-foreground text-xs', busyId ? 'opacity-100' : 'opacity-0')}>Выполняется операция...</div>
      </div>

      <SubscriptionDialog item={editing} open={formOpen} onOpenChange={setFormOpen} onSaved={load} />
      <LogDialog item={logItem} open={!!logItem} onOpenChange={(open) => !open && setLogItem(null)} />
      <PreviewDialog item={previewItem} open={!!previewItem} onOpenChange={(open) => !open && setPreviewItem(null)} />
    </TooltipProvider>
  )
}

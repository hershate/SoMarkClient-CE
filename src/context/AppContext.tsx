import { listen } from '@tauri-apps/api/event'
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import { translations } from '../i18n'
import { backend } from '../lib/tauri'
import type {
  AppSettings,
  NewTask,
  ServiceProfile,
  Task,
  UploadProgress,
  Usage,
} from '../types'

type Toast = { id: number; message: string; tone: 'info' | 'success' | 'error' }

interface AppContextValue {
  ready: boolean
  fatalError: string | null
  version: string
  portable: boolean
  settings: AppSettings | null
  services: ServiceProfile[]
  tasks: Task[]
  usage: Record<string, Usage>
  toasts: Toast[]
  t: (key: string, variables?: Record<string, string | number>) => string
  notify: (message: string, tone?: Toast['tone']) => void
  dismissToast: (id: number) => void
  saveSettings: (settings: AppSettings) => Promise<void>
  saveService: (profile: ServiceProfile, apiKey?: string) => Promise<void>
  deleteService: (id: string) => Promise<void>
  validateService: (id: string) => Promise<Usage>
  setActiveService: (id: string) => Promise<void>
  enqueueTasks: (tasks: NewTask[]) => Promise<Task[]>
  queueAction: (id: string, action: string, confirmed?: boolean, priority?: number) => Promise<void>
  pauseQueue: (paused: boolean) => Promise<void>
  clearHistory: (olderThanDays?: number) => Promise<number>
  refreshTasks: () => Promise<void>
}

const AppContext = createContext<AppContextValue | null>(null)

export function AppProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(false)
  const [fatalError, setFatalError] = useState<string | null>(null)
  const [version, setVersion] = useState('0.1.0')
  const [portable, setPortable] = useState(false)
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [services, setServices] = useState<ServiceProfile[]>([])
  const [tasks, setTasks] = useState<Task[]>([])
  const [usage, setUsage] = useState<Record<string, Usage>>({})
  const [toasts, setToasts] = useState<Toast[]>([])
  const previousStatuses = useRef(new Map<string, string>())
  const toastId = useRef(0)

  const notify = useCallback((message: string, tone: Toast['tone'] = 'info') => {
    const id = ++toastId.current
    setToasts((items) => [...items, { id, message, tone }])
    window.setTimeout(() => setToasts((items) => items.filter((item) => item.id !== id)), 4500)
  }, [])

  const dismissToast = useCallback((id: number) => {
    setToasts((items) => items.filter((item) => item.id !== id))
  }, [])

  const refreshTasks = useCallback(async () => {
    setTasks(await backend.listTasks())
  }, [])

  useEffect(() => {
    let active = true
    backend.bootstrap()
      .then((bootstrap) => {
        if (!active) return
        setVersion(bootstrap.version)
        setPortable(bootstrap.portable)
        setSettings(bootstrap.settings)
        setServices(bootstrap.services)
        setTasks(bootstrap.tasks)
        bootstrap.tasks.forEach((task) => previousStatuses.current.set(task.id, task.status))
        setReady(true)
      })
      .catch((error) => setFatalError(String(error)))
    return () => {
      active = false
    }
  }, [])

  useEffect(() => {
    const subscriptions = [
      listen<Task>('task-updated', ({ payload }) => {
        setTasks((items) => {
          const exists = items.some((item) => item.id === payload.id)
          return exists
            ? items.map((item) => (item.id === payload.id ? payload : item))
            : [payload, ...items]
        })
        const previous = previousStatuses.current.get(payload.id)
        const terminal = ['success', 'partialSuccess', 'failed'].includes(payload.status)
        if (previous && previous !== payload.status && terminal && settings?.sound) {
          playCompletionTone(payload.status === 'failed')
        }
        previousStatuses.current.set(payload.id, payload.status)
      }),
      listen<UploadProgress>('upload-progress', ({ payload }) => {
        setTasks((items) => items.map((item) => (
          item.id === payload.taskId ? { ...item, progress: payload.progress } : item
        )))
      }),
      listen<string>('task-removed', ({ payload }) => {
        setTasks((items) => items.filter((item) => item.id !== payload))
        previousStatuses.current.delete(payload)
      }),
    ]
    Promise.all(subscriptions).then((unlisten) => {
      if (!active) unlisten.forEach((dispose) => dispose())
    })
    let active = true
    return () => {
      active = false
      subscriptions.forEach((subscription) => subscription.then((dispose) => dispose()))
    }
  }, [settings?.sound])

  useEffect(() => {
    if (!settings) return
    const root = document.documentElement
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const apply = () => {
      root.dataset.theme = settings.theme === 'system' ? (media.matches ? 'dark' : 'light') : settings.theme
      root.lang = settings.language
    }
    apply()
    media.addEventListener('change', apply)
    return () => media.removeEventListener('change', apply)
  }, [settings])

  const t = useCallback((key: string, variables?: Record<string, string | number>) => {
    const language = settings?.language ?? 'zh-CN'
    let value = translations[language]?.[key] ?? translations['en-US'][key] ?? key
    Object.entries(variables ?? {}).forEach(([name, replacement]) => {
      value = value.replaceAll(`{${name}}`, String(replacement))
    })
    return value
  }, [settings?.language])

  const saveSettings = useCallback(async (next: AppSettings) => {
    await backend.saveSettings(next)
    setSettings(next)
  }, [])

  const saveService = useCallback(async (profile: ServiceProfile, apiKey?: string) => {
    await backend.saveService(profile, apiKey)
    setServices(await backend.bootstrap().then((value) => value.services))
  }, [])

  const deleteService = useCallback(async (id: string) => {
    await backend.deleteService(id)
    const bootstrap = await backend.bootstrap()
    setServices(bootstrap.services)
    setSettings(bootstrap.settings)
  }, [])

  const validateService = useCallback(async (id: string) => {
    const result = await backend.validateService(id)
    setUsage((items) => ({ ...items, [id]: result }))
    return result
  }, [])

  const setActiveService = useCallback(async (id: string) => {
    if (!settings) return
    const next = { ...settings, activeServiceId: id }
    await backend.saveSettings(next)
    setSettings(next)
  }, [settings])

  const enqueueTasks = useCallback(async (newTasks: NewTask[]) => {
    const created = await backend.enqueueTasks(newTasks)
    setTasks((items) => [...created, ...items])
    created.forEach((task) => previousStatuses.current.set(task.id, task.status))
    return created
  }, [])

  const queueAction = useCallback(async (id: string, action: string, confirmed = false, priority?: number) => {
    await backend.queueAction(id, action, confirmed, priority)
  }, [])

  const pauseQueue = useCallback(async (paused: boolean) => {
    await backend.pauseQueue(paused)
  }, [])

  const clearHistory = useCallback(async (olderThanDays?: number) => {
    const removed = await backend.clearHistory(olderThanDays)
    await refreshTasks()
    return removed
  }, [refreshTasks])

  const value = useMemo<AppContextValue>(() => ({
    ready,
    fatalError,
    version,
    portable,
    settings,
    services,
    tasks,
    usage,
    toasts,
    t,
    notify,
    dismissToast,
    saveSettings,
    saveService,
    deleteService,
    validateService,
    setActiveService,
    enqueueTasks,
    queueAction,
    pauseQueue,
    clearHistory,
    refreshTasks,
  }), [
    ready, fatalError, version, portable, settings, services, tasks, usage, toasts, t, notify,
    dismissToast, saveSettings, saveService, deleteService, validateService, setActiveService,
    enqueueTasks, queueAction, pauseQueue, clearHistory, refreshTasks,
  ])

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>
}

export function useApp() {
  const value = useContext(AppContext)
  if (!value) throw new Error('useApp must be used inside AppProvider')
  return value
}

function playCompletionTone(failed: boolean) {
  try {
    const context = new AudioContext()
    const oscillator = context.createOscillator()
    const gain = context.createGain()
    oscillator.frequency.value = failed ? 240 : 620
    gain.gain.setValueAtTime(0.08, context.currentTime)
    gain.gain.exponentialRampToValueAtTime(0.001, context.currentTime + 0.35)
    oscillator.connect(gain)
    gain.connect(context.destination)
    oscillator.start()
    oscillator.stop(context.currentTime + 0.35)
    oscillator.addEventListener('ended', () => void context.close())
  } catch {
    // System audio can be unavailable until the first user interaction.
  }
}

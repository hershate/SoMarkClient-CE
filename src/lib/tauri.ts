import { invoke } from '@tauri-apps/api/core'
import type {
  AppSettings,
  Bootstrap,
  NewTask,
  OutputFile,
  PreflightResult,
  ServiceProfile,
  StoredResult,
  Task,
  Usage,
} from '../types'

export const backend = {
  bootstrap: () => invoke<Bootstrap>('get_bootstrap'),
  saveService: (profile: ServiceProfile, apiKey?: string) =>
    invoke<void>('save_service', { profile, apiKey }),
  deleteService: (id: string) => invoke<void>('delete_service', { id }),
  saveSettings: (settings: AppSettings) => invoke<void>('save_settings', { settings }),
  validateService: (id: string) => invoke<Usage>('validate_service', { id }),
  enqueueTasks: (newTasks: NewTask[]) => invoke<Task[]>('enqueue_tasks', { newTasks }),
  listTasks: () => invoke<Task[]>('list_tasks'),
  queueAction: (id: string, action: string, confirmed = false, priority?: number) =>
    invoke<void>('queue_action', { id, action, confirmed, priority }),
  pauseQueue: (paused: boolean) => invoke<void>('pause_queue', { paused }),
  loadTaskResult: (id: string) => invoke<StoredResult>('load_task_result', { id }),
  prepareTaskAssets: (id: string) => invoke<void>('prepare_task_assets', { id }),
  resolveResultAsset: (id: string, source: string) =>
    invoke<string>('resolve_result_asset', { id, source }),
  saveMarkdownCopy: (id: string, content: string, destination?: string) =>
    invoke<OutputFile>('save_markdown_copy', { id, content, destination }),
  exportTaskResult: (id: string, format: 'html' | 'txt') =>
    invoke<OutputFile>('export_task_result', { id, format }),
  clearHistory: (olderThanDays?: number) =>
    invoke<number>('clear_history', { olderThanDays }),
  exportDiagnostics: (destination: string) =>
    invoke<string>('export_diagnostics', { destination }),
  preflightFiles: (paths: string[], recursive: boolean, serviceId: string) =>
    invoke<PreflightResult[]>('preflight_files', { paths, recursive, serviceId }),
  importClipboardImage: (serviceId: string) =>
    invoke<PreflightResult>('import_clipboard_image', { serviceId }),
  supportedExtensions: () => invoke<string[]>('supported_extensions'),
}

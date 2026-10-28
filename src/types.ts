export type TaskKind = 'parse' | 'office'
export type TaskMode = 'auto' | 'sync' | 'async'
export type TaskStatus =
  | 'queued'
  | 'uploading'
  | 'submissionUnknown'
  | 'waiting'
  | 'stopped'
  | 'paused'
  | 'downloading'
  | 'success'
  | 'partialSuccess'
  | 'failed'

export type TaskSource =
  | { kind: 'local'; path: string }
  | { kind: 'url'; url: string }
  | { kind: 'clipboard'; path: string }

export interface ElementFormats {
  image: 'file' | 'url' | 'base64' | 'none'
  formula: 'latex' | 'mathml' | 'ascii'
  table: 'markdown' | 'html' | 'image'
  cs: 'image'
}

export interface FeatureConfig {
  enableTextCrossPage: boolean
  enableTableCrossPage: boolean
  enableTitleLevelRecognition: boolean
  enableInlineImage: boolean
  enableTableImage: boolean
  enableImageUnderstanding: boolean
  enableImageOcr: boolean
  keepHeaderFooter: boolean
}

export interface ParseOptions {
  elementFormats: ElementFormats
  featureConfig: FeatureConfig
}

export interface OfficeOptions {
  targetType: 'word' | 'ppt'
  layoutMode: 'flow' | 'layout'
}

export interface AppError {
  kind: string
  message: string
  code: number | null
  retryable: boolean
}

export interface OutputFile {
  format: string
  path: string
}

export interface Task {
  id: string
  batchId: string
  name: string
  kind: TaskKind
  status: TaskStatus
  mode: TaskMode
  source: TaskSource
  formats: string[]
  parseOptions: ParseOptions
  officeOptions: OfficeOptions
  serviceId: string
  remoteId: string | null
  remoteStatus: string | null
  progress: number
  fileSize: number
  pageCount: number | null
  outputRoot: string
  outputs: OutputFile[]
  resultPath: string | null
  error: AppError | null
  retryCount: number
  priority: number
  createdAt: string
  updatedAt: string
  warnings: string[]
}

export interface NewTask {
  name: string
  kind: TaskKind
  mode: TaskMode
  source: TaskSource
  formats: string[]
  parseOptions: ParseOptions
  officeOptions: OfficeOptions
  serviceId: string
  outputRoot: string
  fileSize: number
  pageCount: number | null
}

export interface ServiceProfile {
  id: string
  name: string
  baseUrl: string
  proxyMode: 'system' | 'direct' | 'custom'
  proxyUrl: string | null
  timeoutSecs: number
  maxRetries: number
  maxFileSizeMb: number
  maxPages: number
  qps: number
  maxConcurrency: number
  higherConcurrencyApproved: boolean
}

export interface Usage {
  remainingPaidPages: number
  remainingFreePagesToday: number
  remainingFreePagesThisMonth: number
  dashboardUrl: string
  warnings: string[]
}

export interface AppSettings {
  language: 'zh-CN' | 'en-US'
  theme: 'system' | 'light' | 'dark'
  closeToTray: boolean
  notifications: boolean
  sound: boolean
  historyDays: number
  logDays: number
  defaultOutputRoot: string
  concurrency: number
  retryLimit: number
  quotaWarningPages: number
  blockOnLowQuota: boolean
  activeServiceId: string | null
}

export interface Bootstrap {
  version: string
  portable: boolean
  settings: AppSettings
  services: ServiceProfile[]
  tasks: Task[]
}

export interface CatalogNode {
  page_num: number
  block_idx: number
  title_level: number
  content: string
  children: CatalogNode[]
}

export interface DocumentBlock {
  idx: number
  type: string
  bbox?: [number, number, number, number]
  content?: string
  format?: string
  captions?: number[]
  img_url?: string
  title_level?: number | null
}

export interface DocumentPage {
  page_num: number
  blocks: DocumentBlock[]
  page_size: { w: number; h: number }
}

export interface StructuredJson {
  pages: DocumentPage[]
}

export interface ParseResult {
  fileName: string | null
  catalog: CatalogNode[] | null
  outputs: {
    markdown: string | null
    json: StructuredJson | null
    zip: string | null
  }
}

export interface OfficeResult {
  taskId: string | null
  status: string | null
  resultFileName: string | null
  downloadUrl: string | null
  expiresIn: number | null
}

export interface StoredResult {
  parse: ParseResult | null
  office: OfficeResult | null
}

export interface PreflightResult {
  name: string
  path: string
  size: number
  pageCount: number | null
  accepted: boolean
  issues: string[]
}

export interface UploadProgress {
  taskId: string
  sent: number
  total: number
  progress: number
}

export const defaultParseOptions: ParseOptions = {
  elementFormats: { image: 'file', formula: 'latex', table: 'markdown', cs: 'image' },
  featureConfig: {
    enableTextCrossPage: true,
    enableTableCrossPage: true,
    enableTitleLevelRecognition: true,
    enableInlineImage: true,
    enableTableImage: true,
    enableImageUnderstanding: true,
    enableImageOcr: false,
    keepHeaderFooter: false,
  },
}

export const defaultOfficeOptions: OfficeOptions = {
  targetType: 'word',
  layoutMode: 'flow',
}

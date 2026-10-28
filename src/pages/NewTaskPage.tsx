import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import {
  Clipboard,
  File,
  FilePlus2,
  FolderInput,
  Globe2,
  LoaderCircle,
  Plus,
  Settings2,
  X,
} from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import ParseOptionsPanel from '../components/ParseOptions'
import { Modal, PageHeader, Toggle } from '../components/ui'
import { useApp } from '../context/AppContext'
import { fileNameFromUrl, formatBytes } from '../lib/format'
import { backend } from '../lib/tauri'
import {
  defaultOfficeOptions,
  defaultParseOptions,
  type NewTask,
  type OfficeOptions,
  type ParseOptions,
  type PreflightResult,
  type TaskKind,
  type TaskMode,
  type TaskSource,
} from '../types'

type Overrides = { mode: TaskMode; formats: string[]; options: ParseOptions }
type DraftItem = { id: string; preflight: PreflightResult; source: TaskSource; overrides?: Overrides }
type Preset = { name: string; formats: string[]; options: ParseOptions }

const builtInPresets: Preset[] = [
  { name: '通用 / General', formats: ['markdown', 'json', 'zip'], options: defaultParseOptions },
  {
    name: '论文 / Paper',
    formats: ['markdown', 'json', 'zip'],
    options: defaultParseOptions,
  },
  {
    name: '合同 / Contract',
    formats: ['markdown', 'json', 'zip'],
    options: {
      ...defaultParseOptions,
      elementFormats: { ...defaultParseOptions.elementFormats, table: 'html' },
    },
  },
  {
    name: 'RAG',
    formats: ['markdown', 'json'],
    options: {
      ...defaultParseOptions,
      elementFormats: { ...defaultParseOptions.elementFormats, image: 'none', table: 'markdown' },
      featureConfig: { ...defaultParseOptions.featureConfig, enableInlineImage: false, enableTableImage: false },
    },
  },
]

export default function NewTaskPage() {
  const {
    t,
    settings,
    services,
    usage,
    portable,
    enqueueTasks,
    notify,
  } = useApp()
  const navigate = useNavigate()
  const [kind, setKind] = useState<TaskKind>('parse')
  const [items, setItems] = useState<DraftItem[]>([])
  const [mode, setMode] = useState<TaskMode>('auto')
  const [formats, setFormats] = useState(['markdown', 'json', 'zip'])
  const [options, setOptions] = useState<ParseOptions>(() => structuredClone(defaultParseOptions))
  const [officeOptions, setOfficeOptions] = useState<OfficeOptions>(defaultOfficeOptions)
  const [outputRoot, setOutputRoot] = useState(settings?.defaultOutputRoot ?? '')
  const [recursive, setRecursive] = useState(true)
  const [dragging, setDragging] = useState(false)
  const [extensions, setExtensions] = useState<string[]>([])
  const [urlOpen, setUrlOpen] = useState(false)
  const [url, setUrl] = useState('')
  const [editing, setEditing] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [customPresets, setCustomPresets] = useState<Preset[]>(loadPresets)
  const activeServiceId = settings?.activeServiceId ?? services[0]?.id ?? ''
  const activeUsage = usage[activeServiceId]

  useEffect(() => {
    void backend.supportedExtensions().then(setExtensions)
  }, [])

  useEffect(() => {
    let disposed = false
    const subscription = getCurrentWebview().onDragDropEvent((event) => {
      if (disposed) return
      if (event.payload.type === 'enter' || event.payload.type === 'over') setDragging(true)
      if (event.payload.type === 'leave') setDragging(false)
      if (event.payload.type === 'drop') {
        setDragging(false)
        void addPaths(event.payload.paths)
      }
    })
    return () => {
      disposed = true
      void subscription.then((unlisten) => unlisten())
    }
  }, [activeServiceId, items, recursive])

  const allValid = useMemo(() => items.length > 0 && items.every((item) => (
    item.preflight.accepted && (kind === 'parse' || isPdf(item.source))
  )) && (kind === 'office' || formats.length > 0), [items, kind, formats])

  async function addPaths(paths: string[]) {
    if (!paths.length || !activeServiceId) return
    try {
      const results = await backend.preflightFiles(paths, recursive, activeServiceId)
      addResults(results, 'local')
    } catch (error) {
      notify(String(error), 'error')
    }
  }

  function addResults(results: PreflightResult[], sourceKind: 'local' | 'clipboard') {
    const accepted: DraftItem[] = []
    for (const preflight of results) {
      if (items.some((item) => item.preflight.path === preflight.path) && !window.confirm(t('new.duplicate'))) continue
      accepted.push({
        id: crypto.randomUUID(),
        preflight,
        source: sourceKind === 'local'
          ? { kind: 'local', path: preflight.path }
          : { kind: 'clipboard', path: preflight.path },
      })
    }
    setItems((current) => [...current, ...accepted])
  }

  async function chooseFiles() {
    const selected = await open({
      multiple: true,
      directory: false,
      filters: [{ name: 'Documents', extensions: kind === 'office' ? ['pdf'] : extensions }],
    })
    if (selected) await addPaths(selected)
  }

  async function chooseFolder() {
    const selected = await open({ multiple: false, directory: true })
    if (selected) await addPaths([selected])
  }

  async function chooseOutput() {
    const selected = await open({ multiple: false, directory: true })
    if (selected) setOutputRoot(selected)
  }

  async function importClipboard() {
    try {
      const result = await backend.importClipboardImage(activeServiceId)
      addResults([result], 'clipboard')
    } catch (error) {
      const key = String(error)
      notify(t(key.startsWith('error.') ? key : `error.${key}`), 'error')
    }
  }

  function addUrl() {
    try {
      const parsed = new URL(url)
      if (!['http:', 'https:'].includes(parsed.protocol)) throw new Error('protocol')
      const source: TaskSource = { kind: 'url', url: parsed.toString() }
      if (items.some((item) => item.source.kind === 'url' && item.source.url === parsed.toString()) && !window.confirm(t('new.duplicate'))) return
      setItems((current) => [...current, {
        id: crypto.randomUUID(),
        source,
        preflight: {
          name: fileNameFromUrl(parsed.toString()),
          path: parsed.toString(),
          size: 0,
          pageCount: null,
          accepted: true,
          issues: [],
        },
      }])
      setUrl('')
      setUrlOpen(false)
    } catch {
      notify('Invalid HTTP(S) URL', 'error')
    }
  }

  function applyPreset(name: string) {
    const preset = [...builtInPresets, ...customPresets].find((item) => item.name === name)
    if (!preset) return
    setFormats([...preset.formats])
    setOptions(structuredClone(preset.options))
  }

  function savePreset() {
    const name = window.prompt(settings?.language === 'en-US' ? 'Preset name' : '预设名称')
    if (!name?.trim()) return
    const next = [...customPresets.filter((item) => item.name !== name), {
      name,
      formats: [...formats],
      options: structuredClone(options),
    }]
    setCustomPresets(next)
    localStorage.setItem('somark.presets', JSON.stringify(next))
  }

  async function submit() {
    if (!allValid || busy) return
    if (settings?.blockOnLowQuota && activeUsage) {
      const remaining = activeUsage.remainingPaidPages
        + activeUsage.remainingFreePagesToday
      if (remaining < settings.quotaWarningPages) {
        notify(`Low quota: ${remaining} pages`, 'error')
        return
      }
    }
    setBusy(true)
    try {
      const newTasks: NewTask[] = items.map((item) => ({
        name: item.preflight.name,
        kind,
        mode: item.overrides?.mode ?? mode,
        source: item.source,
        formats: item.overrides?.formats ?? formats,
        parseOptions: item.overrides?.options ?? options,
        officeOptions,
        serviceId: activeServiceId,
        outputRoot,
        fileSize: item.preflight.size,
        pageCount: item.preflight.pageCount,
      }))
      await enqueueTasks(newTasks)
      setItems([])
      navigate('/tasks')
    } catch (error) {
      notify(String(error), 'error')
    } finally {
      setBusy(false)
    }
  }

  const editingItem = items.find((item) => item.id === editing)

  return (
    <div className="page new-task-page">
      <PageHeader title={t('new.title')} subtitle={t('new.subtitle')} />
      <div className="segmented large-segmented">
        <button type="button" className={kind === 'parse' ? 'active' : ''} onClick={() => setKind('parse')}>
          <FilePlus2 size={18} />{t('new.parse')}
        </button>
        <button type="button" className={kind === 'office' ? 'active' : ''} onClick={() => setKind('office')}>
          <File size={18} />{t('new.office')}
        </button>
      </div>
      <div className="new-task-grid">
        <section className="workspace-primary">
          <div className={`dropzone ${dragging ? 'dragging' : ''}`}>
            <div className="dropzone-icon"><Plus size={26} /></div>
            <h2>{t('new.drop')}</h2>
            <p>{kind === 'office' ? 'PDF · 200 MB / 300 pages' : t('new.dropHint')}</p>
            <div className="dropzone-actions">
              <button type="button" className="button primary" onClick={chooseFiles}><FilePlus2 size={16} />{t('new.selectFiles')}</button>
              <button type="button" className="button" onClick={chooseFolder}><FolderInput size={16} />{t('new.selectFolder')}</button>
              <button type="button" className="button ghost" onClick={() => setUrlOpen(true)}><Globe2 size={16} />{t('new.addUrl')}</button>
              <button type="button" className="button ghost" onClick={importClipboard}><Clipboard size={16} />{t('new.clipboard')}</button>
            </div>
            <Toggle checked={recursive} onChange={setRecursive} label={t('new.recursive')} />
          </div>
          <div className="panel file-panel">
            <div className="panel-heading">
              <div><h2>{t('new.selected')}</h2><span className="count-badge">{items.length}</span></div>
              {items.length > 0 && <button type="button" className="text-button" onClick={() => setItems([])}>{t('new.clear')}</button>}
            </div>
            {items.length === 0 ? (
              <div className="file-empty"><File size={25} /><span>{t('new.noFiles')}</span></div>
            ) : (
              <div className="file-list">
                {items.map((item) => {
                  const pdfProblem = kind === 'office' && !isPdf(item.source)
                  return (
                    <div className={`file-row ${!item.preflight.accepted || pdfProblem ? 'invalid' : ''}`} key={item.id}>
                      <div className="file-type-icon">{item.preflight.name.split('.').pop()?.slice(0, 4).toUpperCase()}</div>
                      <div className="file-info">
                        <strong title={item.preflight.path}>{item.preflight.name}</strong>
                        <span>{formatBytes(item.preflight.size)}{item.preflight.pageCount ? ` · ${item.preflight.pageCount} pages` : ''}</span>
                        {(item.preflight.issues.length > 0 || pdfProblem) && (
                          <small>{pdfProblem ? 'PDF only' : item.preflight.issues.map((issue) => t(`error.${issue}`)).join(' · ')}</small>
                        )}
                      </div>
                      {item.overrides && <span className="override-badge">Custom</span>}
                      <button type="button" className="icon-button" title="Override options" onClick={() => setEditing(item.id)}><Settings2 size={16} /></button>
                      <button type="button" className="icon-button danger" onClick={() => setItems((current) => current.filter((candidate) => candidate.id !== item.id))}><X size={16} /></button>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        </section>
        <aside className="panel options-panel">
          <div className="panel-heading"><h2>{t('new.options')}</h2></div>
          <div className="field">
            <label htmlFor="task-mode">{t('new.mode')}</label>
            <select id="task-mode" value={mode} onChange={(event) => setMode(event.target.value as TaskMode)}>
              <option value="auto">{t('new.auto')}</option>
              <option value="sync">{t('new.sync')}</option>
              <option value="async">{t('new.async')}</option>
            </select>
          </div>
          {kind === 'parse' ? (
            <>
              <div className="field">
                <label htmlFor="preset">Preset</label>
                <div className="input-action">
                  <select id="preset" defaultValue="" onChange={(event) => applyPreset(event.target.value)}>
                    <option value="" disabled>Choose preset</option>
                    {[...builtInPresets, ...customPresets].map((preset) => <option key={preset.name}>{preset.name}</option>)}
                  </select>
                  <button type="button" className="button small" onClick={savePreset}>+</button>
                </div>
              </div>
              <ParseOptionsPanel options={options} onChange={setOptions} formats={formats} onFormatsChange={setFormats} />
            </>
          ) : (
            <>
              <div className="field">
                <label htmlFor="office-target">{t('new.target')}</label>
                <select id="office-target" value={officeOptions.targetType} onChange={(event) => {
                  const targetType = event.target.value as OfficeOptions['targetType']
                  setOfficeOptions({ targetType, layoutMode: targetType === 'ppt' ? 'layout' : 'flow' })
                }}>
                  <option value="word">{t('new.word')}</option>
                  <option value="ppt">{t('new.ppt')}</option>
                </select>
              </div>
              <div className="field">
                <label htmlFor="office-layout">{t('new.layout')}</label>
                <select id="office-layout" value={officeOptions.layoutMode} disabled={officeOptions.targetType === 'ppt'} onChange={(event) => setOfficeOptions({ ...officeOptions, layoutMode: event.target.value as OfficeOptions['layoutMode'] })}>
                  {officeOptions.targetType === 'word' && <option value="flow">{t('new.flow')}</option>}
                  <option value="layout">{t('new.layoutMode')}</option>
                </select>
              </div>
            </>
          )}
          <div className="field">
            <label htmlFor="output-root">{t('new.outputRoot')}</label>
            <div className="input-action">
              <input id="output-root" value={outputRoot} onChange={(event) => setOutputRoot(event.target.value)} />
              <button type="button" className="button small" onClick={chooseOutput}>…</button>
            </div>
            <small>{t('new.outputHint')}</small>
          </div>
          {portable && <p className="inline-notice">{t('services.portableKeyHint')}</p>}
          <button type="button" className="button primary submit-button" disabled={!allValid || busy || !outputRoot.trim()} onClick={submit}>
            {busy ? <LoaderCircle className="spin" size={17} /> : <Plus size={17} />}
            {t('new.submit')}
          </button>
          {!allValid && items.length > 0 && <small className="validation-hint">{t('new.invalid')}</small>}
        </aside>
      </div>
      {urlOpen && (
        <Modal title={t('new.urlTitle')} onClose={() => setUrlOpen(false)}>
          <div className="modal-body">
            <input autoFocus value={url} onChange={(event) => setUrl(event.target.value)} placeholder={t('new.urlPlaceholder')} onKeyDown={(event) => { if (event.key === 'Enter') addUrl() }} />
            <p className="muted">{t('new.offlineOnly')}</p>
          </div>
          <div className="modal-footer"><button type="button" className="button" onClick={() => setUrlOpen(false)}>{t('common.cancel')}</button><button type="button" className="button primary" onClick={addUrl}>{t('common.confirm')}</button></div>
        </Modal>
      )}
      {editingItem && (
        <OverrideEditor
          item={editingItem}
          kind={kind}
          fallback={{ mode, formats, options }}
          onClose={() => setEditing(null)}
          onSave={(overrides) => {
            setItems((current) => current.map((item) => item.id === editingItem.id ? { ...item, overrides } : item))
            setEditing(null)
          }}
        />
      )}
    </div>
  )
}

function OverrideEditor({ item, kind, fallback, onClose, onSave }: {
  item: DraftItem
  kind: TaskKind
  fallback: Overrides
  onClose: () => void
  onSave: (overrides: Overrides) => void
}) {
  const { t } = useApp()
  const [mode, setMode] = useState(item.overrides?.mode ?? fallback.mode)
  const [formats, setFormats] = useState(item.overrides?.formats ?? fallback.formats)
  const [options, setOptions] = useState(() => structuredClone(item.overrides?.options ?? fallback.options))
  return (
    <Modal title={`${t('new.options')} · ${item.preflight.name}`} onClose={onClose} wide>
      <div className="modal-body">
        <div className="field">
          <label>{t('new.mode')}</label>
          <select value={mode} onChange={(event) => setMode(event.target.value as TaskMode)}>
            <option value="auto">{t('new.auto')}</option><option value="sync">{t('new.sync')}</option><option value="async">{t('new.async')}</option>
          </select>
        </div>
        {kind === 'parse' && <ParseOptionsPanel options={options} onChange={setOptions} formats={formats} onFormatsChange={setFormats} />}
      </div>
      <div className="modal-footer"><button type="button" className="button" onClick={onClose}>{t('common.cancel')}</button><button type="button" className="button primary" disabled={kind === 'parse' && formats.length === 0} onClick={() => onSave({ mode, formats, options })}>{t('common.save')}</button></div>
    </Modal>
  )
}

function isPdf(source: TaskSource) {
  const value = source.kind === 'url' ? new URL(source.url).pathname : source.path
  return value.toLowerCase().endsWith('.pdf')
}

function loadPresets(): Preset[] {
  try {
    return JSON.parse(localStorage.getItem('somark.presets') ?? '[]') as Preset[]
  } catch {
    return []
  }
}

import { openPath } from '@tauri-apps/plugin-opener'
import {
  ArrowLeft,
  Braces,
  Code2,
  Download,
  FileCode2,
  FileText,
  Image,
  ListTree,
  Save,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import JsonTree from '../components/JsonTree'
import MarkdownView from '../components/MarkdownView'
import SourceCompare, { type BlockSelection } from '../components/SourceCompare'
import { Loading, PageHeader, StatusBadge } from '../components/ui'
import { useApp } from '../context/AppContext'
import { backend } from '../lib/tauri'
import type { CatalogNode, StoredResult } from '../types'

type View = 'markdown' | 'source' | 'json' | 'raw' | 'compare'

export default function ResultPage() {
  const { taskId } = useParams()
  const { tasks, t, notify, refreshTasks } = useApp()
  const navigate = useNavigate()
  const [result, setResult] = useState<StoredResult | null>(null)
  const [view, setView] = useState<View>('markdown')
  const [selection, setSelection] = useState<BlockSelection | null>(null)
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const task = tasks.find((item) => item.id === taskId)
  const parse = result?.parse

  useEffect(() => {
    if (!taskId) return
    let active = true
    setResult(null)
    Promise.all([backend.loadTaskResult(taskId), backend.prepareTaskAssets(taskId)])
      .then(([stored]) => {
        if (!active) return
        setResult(stored)
        setDraft(stored.parse?.outputs.markdown ?? '')
      })
      .catch((error) => notify(String(error), 'error'))
    return () => { active = false }
  }, [taskId, notify])

  useEffect(() => {
    if (!selection) return
    document.querySelector(`[data-block-id="${selection.page}-${selection.block}"]`)
      ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }, [selection])

  async function saveCopy() {
    if (!taskId || busy) return
    setBusy(true)
    try {
      const output = await backend.saveMarkdownCopy(taskId, draft)
      await refreshTasks()
      setEditing(false)
      notify(`${t('common.saved')} · ${output.path}`, 'success')
    } catch (error) {
      notify(String(error), 'error')
    } finally {
      setBusy(false)
    }
  }

  async function exportResult(format: 'html' | 'txt') {
    if (!taskId || busy) return
    setBusy(true)
    try {
      const output = await backend.exportTaskResult(taskId, format)
      await refreshTasks()
      notify(`${output.format.toUpperCase()} · ${output.path}`, 'success')
    } catch (error) {
      notify(String(error), 'error')
    } finally {
      setBusy(false)
    }
  }

  const views: { id: View; label: string; icon: typeof FileText }[] = [
    { id: 'markdown', label: t('preview.markdown'), icon: FileText },
    { id: 'source', label: t('preview.source'), icon: Code2 },
    { id: 'json', label: t('preview.jsonTree'), icon: ListTree },
    { id: 'raw', label: t('preview.rawJson'), icon: Braces },
    { id: 'compare', label: t('preview.compare'), icon: Image },
  ]

  return (
    <div className="page result-page">
      <PageHeader
        title={task?.name ?? t('tasks.open')}
        subtitle={task ? `${task.kind === 'parse' ? 'Document parsing' : 'PDF to Office'} · ${task.outputs.length} files` : ''}
        actions={<div className="result-actions">
          {parse && <button type="button" className="button" disabled={busy} onClick={() => void exportResult('html')}><FileCode2 size={15} />{t('preview.exportHtml')}</button>}
          {parse && <button type="button" className="button" disabled={busy} onClick={() => void exportResult('txt')}><Download size={15} />{t('preview.exportText')}</button>}
          <button type="button" className="button" onClick={() => navigate(-1)}><ArrowLeft size={16} />{t('common.close')}</button>
        </div>}
      />
      {!result ? <Loading /> : (
        <div className="result-workspace">
          <aside className="result-sidebar">
            {task && <div className="result-status"><StatusBadge status={task.status} /></div>}
            {!!parse?.catalog?.length && (
              <div className="catalog-panel">
                <h3>{t('preview.catalog')}</h3>
                <CatalogTree nodes={parse.catalog} onSelect={(next) => { setSelection(next); setView('compare') }} />
              </div>
            )}
            <div className="output-panel">
              <h3>Files</h3>
              {task?.outputs.map((output, index) => (
                <button type="button" key={`${output.path}-${index}`} title={output.path} onClick={() => void openPath(output.path)}>
                  <FileText size={14} /><span>{output.format}</span>
                </button>
              ))}
            </div>
          </aside>
          <section className="result-main panel">
            {parse ? (
              <>
                <div className="result-tabs" role="tablist">
                  {views.map(({ id, label, icon: Icon }) => (
                    <button type="button" role="tab" aria-selected={view === id} key={id} className={view === id ? 'active' : ''} onClick={() => { setView(id); setEditing(false) }}><Icon size={15} />{label}</button>
                  ))}
                  {view === 'source' && <button type="button" className="edit-result-button" onClick={() => editing ? void saveCopy() : setEditing(true)}><Save size={14} />{t(editing ? 'preview.save' : 'preview.edit')}</button>}
                </div>
                <div className="result-view">
                  {view === 'markdown' && taskId && <MarkdownView taskId={taskId} markdown={parse.outputs.markdown ?? ''} />}
                  {view === 'source' && (editing
                    ? <textarea className="markdown-editor" value={draft} onChange={(event) => setDraft(event.target.value)} spellCheck={false} />
                    : <pre className="markdown-source">{parse.outputs.markdown}</pre>)}
                  {view === 'json' && <JsonTree value={parse.outputs.json} />}
                  {view === 'raw' && <pre className="markdown-source">{JSON.stringify(parse.outputs.json, null, 2)}</pre>}
                  {view === 'compare' && task && <SourceCompare task={task} json={parse.outputs.json} selection={selection} onSelect={setSelection} />}
                </div>
              </>
            ) : (
              <div className="office-result">
                <div className="success-seal">✓</div>
                <h2>{result.office?.resultFileName ?? task?.name}</h2>
                <p>{result.office?.status}</p>
                {task?.outputs.map((output) => <button type="button" className="button primary" key={output.path} onClick={() => void openPath(output.path)}><Download size={16} />{output.format}</button>)}
                <JsonTree value={result.office} />
              </div>
            )}
          </section>
        </div>
      )}
    </div>
  )
}

function CatalogTree({ nodes, onSelect }: { nodes: CatalogNode[]; onSelect: (selection: BlockSelection) => void }) {
  return (
    <div className="catalog-tree">
      {nodes.map((node, index) => <CatalogItem key={`${node.page_num}-${node.block_idx}-${index}`} node={node} onSelect={onSelect} />)}
    </div>
  )
}

function CatalogItem({ node, onSelect }: { node: CatalogNode; onSelect: (selection: BlockSelection) => void }) {
  const [open, setOpen] = useState(true)
  return (
    <div className="catalog-item">
      <button type="button" style={{ paddingLeft: `${(node.title_level - 1) * 10 + 4}px` }} onClick={() => {
        if (node.children.length) setOpen(!open)
        onSelect({ page: node.page_num, block: node.block_idx })
      }}>
        <span>{node.content}</span><small>{node.page_num + 1}</small>
      </button>
      {open && node.children.length > 0 && <CatalogTree nodes={node.children} onSelect={onSelect} />}
    </div>
  )
}

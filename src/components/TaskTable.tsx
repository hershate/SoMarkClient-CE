import { openPath } from '@tauri-apps/plugin-opener'
import {
  ArrowDown,
  ArrowUp,
  Eye,
  FileOutput,
  FileText,
  FolderOpen,
  Pause,
  Play,
  RotateCcw,
  Square,
  Trash2,
} from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useApp } from '../context/AppContext'
import { elapsed, formatBytes, formatDate } from '../lib/format'
import type { Task } from '../types'
import { ProgressBar, StatusBadge } from './ui'

export default function TaskTable({ items, history = false }: { items: Task[]; history?: boolean }) {
  const {
    t,
    settings,
    services,
    queueAction,
    refreshTasks,
    notify,
  } = useApp()
  const navigate = useNavigate()

  async function action(task: Task, name: string, priority?: number) {
    try {
      const confirmed = name === 'retry' && task.status === 'submissionUnknown'
        ? window.confirm(t('tasks.confirmUnknown'))
        : name === 'retry'
      if (name === 'retry' && !confirmed) return
      await queueAction(task.id, name, confirmed, priority)
      if (name === 'reorder') await refreshTasks()
    } catch (error) {
      notify(String(error), 'error')
    }
  }

  async function openFolder(task: Task) {
    const first = task.outputs[0]
    if (!first) return
    const path = first.path.replace(/[\\/][^\\/]*$/, '')
    await openPath(path).catch((error) => notify(String(error), 'error'))
  }

  return (
    <div className="task-table-wrap">
      <table className="task-table">
        <thead>
          <tr>
            <th>{t('tasks.name')}</th>
            <th>{t('tasks.status')}</th>
            <th>{t('tasks.progress')}</th>
            <th>{t('tasks.service')}</th>
            <th>{t('tasks.updated')}</th>
            <th className="align-right">{t('tasks.actions')}</th>
          </tr>
        </thead>
        <tbody>
          {items.map((task) => {
            const active = ['queued', 'uploading', 'waiting', 'downloading'].includes(task.status)
            const canOpen = task.resultPath && (task.status === 'success' || task.status === 'partialSuccess')
            return (
              <tr key={task.id}>
                <td>
                  <div className="task-name-cell">
                    <div className="task-kind">{task.kind === 'parse' ? <FileText size={18} /> : <FileOutput size={18} />}</div>
                    <div>
                      <strong title={task.name}>{task.name}</strong>
                      <span>{formatBytes(task.fileSize)}{task.pageCount ? ` · ${task.pageCount}p` : ''} · {elapsed(task.createdAt, task.updatedAt)}</span>
                      {task.remoteId && <small title={task.remoteId}>ID {task.remoteId.slice(0, 12)}…</small>}
                      {task.error && <small className="error-text" title={task.error.message}>{task.error.message}</small>}
                    </div>
                  </div>
                </td>
                <td>
                  <StatusBadge status={task.status} />
                  {task.retryCount > 0 && <div className="retry-label">{t('tasks.retryCount', { count: task.retryCount })}</div>}
                </td>
                <td className="progress-cell">
                  <ProgressBar value={task.progress} indeterminate={task.status === 'waiting'} />
                  <span>{task.status === 'uploading' ? `${Math.round(task.progress * 100)}%` : task.remoteStatus ?? task.mode}</span>
                </td>
                <td><span className="service-name">{services.find((service) => service.id === task.serviceId)?.name ?? '—'}</span></td>
                <td><span className="date-cell">{formatDate(task.updatedAt, settings?.language ?? 'zh-CN')}</span></td>
                <td>
                  <div className="row-actions">
                    {canOpen && <button type="button" className="icon-button emphasized" title={t('tasks.open')} onClick={() => navigate(`/result/${task.id}`)}><Eye size={16} /></button>}
                    {task.outputs.length > 0 && <button type="button" className="icon-button" title={t('tasks.openFolder')} onClick={() => void openFolder(task)}><FolderOpen size={16} /></button>}
                    {active && <button type="button" className="icon-button" title={t('tasks.pause')} onClick={() => void action(task, 'pause')}><Pause size={16} /></button>}
                    {(task.status === 'paused' || task.status === 'stopped') && <button type="button" className="icon-button" title={t('tasks.resume')} onClick={() => void action(task, 'resume')}><Play size={16} /></button>}
                    {['uploading', 'waiting', 'downloading'].includes(task.status) && <button type="button" className="icon-button" title={t('tasks.stop')} onClick={() => void action(task, 'stop')}><Square size={15} /></button>}
                    {['failed', 'partialSuccess', 'submissionUnknown'].includes(task.status) && <button type="button" className={`icon-button ${task.status === 'submissionUnknown' ? 'warning' : ''}`} title={task.status === 'submissionUnknown' ? t('tasks.confirmRetry') : t('tasks.retry')} onClick={() => void action(task, 'retry')}><RotateCcw size={16} /></button>}
                    {!history && !active && <>
                      <button type="button" className="icon-button subtle" title="Priority up" onClick={() => void action(task, 'reorder', task.priority + 1)}><ArrowUp size={15} /></button>
                      <button type="button" className="icon-button subtle" title="Priority down" onClick={() => void action(task, 'reorder', task.priority - 1)}><ArrowDown size={15} /></button>
                    </>}
                    {(!active || task.status === 'stopped') && <button type="button" className="icon-button danger" title={t('tasks.remove')} onClick={() => void action(task, 'remove')}><Trash2 size={16} /></button>}
                  </div>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

import { Search, Trash2 } from 'lucide-react'
import { useMemo, useState } from 'react'
import TaskTable from '../components/TaskTable'
import { EmptyState, PageHeader } from '../components/ui'
import { useApp } from '../context/AppContext'

export default function HistoryPage() {
  const { t, tasks, clearHistory, notify } = useApp()
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState('all')
  const history = useMemo(() => tasks.filter((task) => {
    if (!['success', 'partialSuccess', 'failed'].includes(task.status)) return false
    if (status !== 'all' && task.status !== status) return false
    return `${task.name} ${task.remoteId ?? ''}`.toLowerCase().includes(search.toLowerCase())
  }), [tasks, search, status])

  return (
    <div className="page">
      <PageHeader
        title={t('history.title')}
        subtitle={t('history.subtitle')}
        actions={<button type="button" className="button danger-outline" onClick={async () => {
          if (!window.confirm(t('history.clearHint'))) return
          try {
            await clearHistory()
          } catch (error) { notify(String(error), 'error') }
        }}><Trash2 size={16} />{t('history.clear')}</button>}
      />
      <div className="list-toolbar">
        <label className="search-box wide"><Search size={16} /><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t('history.search')} /></label>
        <select value={status} onChange={(event) => setStatus(event.target.value)}>
          <option value="all">{t('history.all')}</option>
          <option value="success">{t('status.success')}</option>
          <option value="partialSuccess">{t('status.partialSuccess')}</option>
          <option value="failed">{t('status.failed')}</option>
        </select>
      </div>
      {history.length === 0 ? <EmptyState title={t('history.empty')} /> : <TaskTable items={history} history />}
    </div>
  )
}

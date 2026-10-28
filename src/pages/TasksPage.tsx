import { ListFilter, Pause, Play } from 'lucide-react'
import { useMemo, useState } from 'react'
import TaskTable from '../components/TaskTable'
import { EmptyState, PageHeader } from '../components/ui'
import { useApp } from '../context/AppContext'

export default function TasksPage() {
  const { t, tasks, pauseQueue, notify } = useApp()
  const [paused, setPaused] = useState(false)
  const [filter, setFilter] = useState('all')
  const [search, setSearch] = useState('')
  const active = tasks.filter((task) => ['queued', 'uploading', 'waiting', 'downloading'].includes(task.status)).length
  const issues = tasks.filter((task) => ['failed', 'partialSuccess', 'submissionUnknown'].includes(task.status)).length
  const completed = tasks.filter((task) => task.status === 'success').length
  const visible = useMemo(() => tasks.filter((task) => {
    if (search && !`${task.name} ${task.remoteId ?? ''}`.toLowerCase().includes(search.toLowerCase())) return false
    if (filter === 'active') return ['queued', 'uploading', 'waiting', 'downloading'].includes(task.status)
    if (filter === 'issues') return ['failed', 'partialSuccess', 'submissionUnknown'].includes(task.status)
    if (filter === 'completed') return task.status === 'success'
    return true
  }), [tasks, filter, search])

  return (
    <div className="page">
      <PageHeader
        title={t('tasks.title')}
        subtitle={t('tasks.subtitle')}
        actions={<button type="button" className="button" onClick={async () => {
          try {
            await pauseQueue(!paused)
            setPaused(!paused)
          } catch (error) { notify(String(error), 'error') }
        }}>{paused ? <Play size={16} /> : <Pause size={16} />}{t(paused ? 'tasks.resumeAll' : 'tasks.pauseAll')}</button>}
      />
      <div className="metric-grid">
        <div className="metric-card"><span>{t('tasks.active')}</span><strong>{active}</strong><i className="metric-dot green" /></div>
        <div className="metric-card"><span>{t('tasks.completed')}</span><strong>{completed}</strong><i className="metric-dot blue" /></div>
        <div className="metric-card"><span>Attention</span><strong>{issues}</strong><i className="metric-dot amber" /></div>
      </div>
      <div className="list-toolbar">
        <div className="filter-tabs">
          {['all', 'active', 'issues', 'completed'].map((value) => (
            <button type="button" key={value} className={filter === value ? 'active' : ''} onClick={() => setFilter(value)}>
              {value === 'all' ? 'All' : value === 'issues' ? 'Attention' : t(`tasks.${value}`)}
            </button>
          ))}
        </div>
        <label className="search-box"><ListFilter size={16} /><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t('history.search')} /></label>
      </div>
      {visible.length === 0 ? <EmptyState title={t('tasks.empty')} hint={t('tasks.emptyHint')} /> : <TaskTable items={visible} />}
    </div>
  )
}

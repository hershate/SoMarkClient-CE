import { ArrowLeft } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import ReactMarkdown from 'react-markdown'
import rehypeKatex from 'rehype-katex'
import rehypeSanitize from 'rehype-sanitize'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import { Loading, PageHeader } from '../components/ui'
import { useApp } from '../context/AppContext'
import { backend } from '../lib/tauri'
import type { StoredResult } from '../types'

export default function ResultPage() {
  const { taskId } = useParams()
  const { tasks, t, notify } = useApp()
  const navigate = useNavigate()
  const [result, setResult] = useState<StoredResult | null>(null)
  const task = tasks.find((item) => item.id === taskId)

  useEffect(() => {
    if (!taskId) return
    void backend.loadTaskResult(taskId).then(setResult).catch((error) => notify(String(error), 'error'))
  }, [taskId, notify])

  return (
    <div className="page result-page">
      <PageHeader title={task?.name ?? t('tasks.open')} subtitle={t('preview.markdown')} actions={<button type="button" className="button" onClick={() => navigate(-1)}><ArrowLeft size={16} />{t('common.close')}</button>} />
      {!result ? <Loading /> : result.parse?.outputs.markdown ? (
        <article className="panel markdown-body"><ReactMarkdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[rehypeSanitize, rehypeKatex]}>{result.parse.outputs.markdown}</ReactMarkdown></article>
      ) : <pre className="panel raw-result">{JSON.stringify(result.office, null, 2)}</pre>}
    </div>
  )
}

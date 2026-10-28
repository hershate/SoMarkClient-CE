import { convertFileSrc } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useEffect, useState, type AnchorHTMLAttributes, type ImgHTMLAttributes } from 'react'
import ReactMarkdown from 'react-markdown'
import rehypeKatex from 'rehype-katex'
import rehypeSanitize from 'rehype-sanitize'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import { backend } from '../lib/tauri'

export default function MarkdownView({ taskId, markdown }: { taskId: string; markdown: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        rehypePlugins={[rehypeSanitize, rehypeKatex]}
        components={{
          img: (props) => <AssetImage taskId={taskId} {...props} />,
          a: SafeLink,
        }}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  )
}

function AssetImage({ taskId, src, alt, title }: ImgHTMLAttributes<HTMLImageElement> & { taskId: string }) {
  const [resolved, setResolved] = useState<string>()
  const [failed, setFailed] = useState(false)
  useEffect(() => {
    let active = true
    setResolved(undefined)
    setFailed(false)
    if (!src) return
    if (/^(https?|data|blob|asset):/i.test(src)) {
      setResolved(src)
      return
    }
    backend.resolveResultAsset(taskId, src)
      .then((path) => {
        if (active) setResolved(convertFileSrc(path))
      })
      .catch(() => {
        if (active) setFailed(true)
      })
    return () => { active = false }
  }, [taskId, src])
  if (failed) return <span className="missing-image">[{alt || 'image'}]</span>
  if (!resolved) return <span className="image-placeholder" />
  return <img src={resolved} alt={alt ?? ''} title={title} loading="lazy" />
}

function SafeLink({ href, children }: AnchorHTMLAttributes<HTMLAnchorElement>) {
  return (
    <a href={href} onClick={(event) => {
      if (href && /^https?:\/\//i.test(href)) {
        event.preventDefault()
        void openUrl(href)
      }
    }}>
      {children}
    </a>
  )
}

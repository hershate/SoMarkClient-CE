import { convertFileSrc } from '@tauri-apps/api/core'
import { openPath, openUrl } from '@tauri-apps/plugin-opener'
import { ChevronLeft, ChevronRight, ExternalLink, FileWarning } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Document, Page, pdfjs } from 'react-pdf'
import { useApp } from '../context/AppContext'
import type { StructuredJson, Task } from '../types'
import { Loading } from './ui'

pdfjs.GlobalWorkerOptions.workerSrc = new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url).toString()

export type BlockSelection = { page: number; block: number }

export default function SourceCompare({ task, json, selection, onSelect }: {
  task: Task
  json: StructuredJson | null
  selection: BlockSelection | null
  onSelect: (selection: BlockSelection) => void
}) {
  const { t } = useApp()
  const [pageNumber, setPageNumber] = useState((selection?.page ?? 0) + 1)
  const [pageCount, setPageCount] = useState(0)
  const [sourceUrl, setSourceUrl] = useState<string | null>(null)
  const [error, setError] = useState(false)
  const [width, setWidth] = useState(680)
  const container = useRef<HTMLDivElement>(null)
  const path = task.source.kind === 'url' ? null : task.source.path
  const extension = path?.split('.').pop()?.toLowerCase()
  const isPdf = extension === 'pdf'
  const isImage = ['png', 'jpg', 'jpeg', 'bmp', 'gif', 'webp', 'tiff'].includes(extension ?? '')
  const page = json?.pages.find((item) => item.page_num === pageNumber - 1)

  useEffect(() => {
    if (path) setSourceUrl(convertFileSrc(path))
  }, [path])

  useEffect(() => {
    if (selection) setPageNumber(selection.page + 1)
  }, [selection])

  useEffect(() => {
    if (!container.current) return
    const observer = new ResizeObserver(([entry]) => setWidth(Math.max(280, Math.min(820, entry.contentRect.width - 4))))
    observer.observe(container.current)
    return () => observer.disconnect()
  }, [])

  function choosePage(next: number) {
    setPageNumber(next)
    const first = json?.pages.find((item) => item.page_num === next - 1)?.blocks[0]
    if (first) onSelect({ page: next - 1, block: first.idx })
  }

  return (
    <div className="compare-grid">
      <section className="source-pane" ref={container}>
        <div className="preview-toolbar">
          <strong>{t('preview.compare')}</strong>
          {(isPdf || (json?.pages.length ?? 0) > 1) && (
            <div className="page-controls">
              <button type="button" className="icon-button" disabled={pageNumber <= 1} onClick={() => choosePage(pageNumber - 1)}><ChevronLeft size={17} /></button>
              <span>{t('preview.page', { page: pageNumber })} / {pageCount || json?.pages.length || 1}</span>
              <button type="button" className="icon-button" disabled={pageNumber >= (pageCount || json?.pages.length || 1)} onClick={() => choosePage(pageNumber + 1)}><ChevronRight size={17} /></button>
            </div>
          )}
        </div>
        {!path ? (
          <Unavailable text="Remote source" onClick={() => task.source.kind === 'url' && void openUrl(task.source.url)} />
        ) : isPdf && sourceUrl ? (
          <div className="pdf-stage">
            <Document file={sourceUrl} loading={<Loading />} onLoadSuccess={({ numPages }) => setPageCount(numPages)} onLoadError={() => setError(true)}>
              {!error && (
                <div className="pdf-page-wrap" style={{ width }}>
                  <Page pageNumber={pageNumber} width={width} renderTextLayer={false} renderAnnotationLayer={false} />
                  {page?.page_size && <BBoxOverlays page={page} selected={selection} onSelect={onSelect} />}
                </div>
              )}
            </Document>
            {error && <Unavailable text="Unable to render PDF" />}
          </div>
        ) : isImage && sourceUrl ? (
          <div className="image-stage" style={{ width }}>
            <img src={sourceUrl} alt={task.name} />
            {page?.page_size && <BBoxOverlays page={page} selected={selection} onSelect={onSelect} />}
          </div>
        ) : (
          <Unavailable text="Open the source file externally" onClick={() => path && void openPath(path)} />
        )}
      </section>
      <section className="blocks-pane">
        <div className="preview-toolbar"><strong>{t('preview.jsonTree')}</strong><span>{page?.blocks.length ?? 0} blocks</span></div>
        <div className="block-list">
          {page?.blocks.map((block) => (
            <button
              type="button"
              key={block.idx}
              data-block-id={`${page.page_num}-${block.idx}`}
              className={selection?.page === page.page_num && selection.block === block.idx ? 'active' : ''}
              onClick={() => onSelect({ page: page.page_num, block: block.idx })}
            >
              <span>{block.type}{block.title_level ? ` · H${block.title_level}` : ''}</span>
              <p>{block.content || block.img_url || '—'}</p>
              <small>#{block.idx}{block.bbox ? ` · ${block.bbox.join(', ')}` : ''}</small>
            </button>
          ))}
        </div>
      </section>
    </div>
  )
}

function BBoxOverlays({ page, selected, onSelect }: {
  page: NonNullable<StructuredJson['pages'][number]>
  selected: BlockSelection | null
  onSelect: (selection: BlockSelection) => void
}) {
  return (
    <div className="bbox-layer">
      {page.blocks.filter((block) => block.bbox?.length === 4).map((block) => {
        const [x1, y1, x2, y2] = block.bbox!
        const active = selected?.page === page.page_num && selected.block === block.idx
        return (
          <button
            type="button"
            key={block.idx}
            className={`bbox-overlay ${active ? 'active' : ''}`}
            title={block.content || block.type}
            style={{
              left: `${x1 / page.page_size.w * 100}%`,
              top: `${y1 / page.page_size.h * 100}%`,
              width: `${(x2 - x1) / page.page_size.w * 100}%`,
              height: `${(y2 - y1) / page.page_size.h * 100}%`,
            }}
            onClick={() => onSelect({ page: page.page_num, block: block.idx })}
          />
        )
      })}
    </div>
  )
}

function Unavailable({ text, onClick }: { text: string; onClick?: () => void }) {
  return <div className="unavailable"><FileWarning size={30} /><p>{text}</p>{onClick && <button type="button" className="button" onClick={onClick}><ExternalLink size={15} />Open</button>}</div>
}

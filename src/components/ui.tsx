import { Inbox, LoaderCircle, X } from 'lucide-react'
import { useEffect, type ReactNode } from 'react'
import { useApp } from '../context/AppContext'
import type { TaskStatus } from '../types'

export function PageHeader({ title, subtitle, actions }: {
  title: string
  subtitle: string
  actions?: ReactNode
}) {
  return (
    <header className="page-header">
      <div>
        <h1>{title}</h1>
        <p>{subtitle}</p>
      </div>
      {actions && <div className="page-actions">{actions}</div>}
    </header>
  )
}

export function StatusBadge({ status }: { status: TaskStatus }) {
  const { t } = useApp()
  return <span className={`status-badge status-${status}`}>{t(`status.${status}`)}</span>
}

export function Toggle({ checked, onChange, label, disabled = false }: {
  checked: boolean
  onChange: (checked: boolean) => void
  label: string
  disabled?: boolean
}) {
  return (
    <label className={`toggle-row ${disabled ? 'disabled' : ''}`}>
      <button
        type="button"
        className={`toggle ${checked ? 'on' : ''}`}
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
      >
        <span />
      </button>
      <span>{label}</span>
    </label>
  )
}

export function Modal({ title, children, onClose, wide = false }: {
  title: string
  children: ReactNode
  onClose: () => void
  wide?: boolean
}) {
  useEffect(() => {
    const close = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', close)
    return () => window.removeEventListener('keydown', close)
  }, [onClose])

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose()
    }}>
      <section className={`modal ${wide ? 'modal-wide' : ''}`} role="dialog" aria-modal="true" aria-label={title}>
        <div className="modal-header">
          <h2>{title}</h2>
          <button type="button" className="icon-button" onClick={onClose} aria-label="Close">
            <X size={18} />
          </button>
        </div>
        {children}
      </section>
    </div>
  )
}

export function EmptyState({ title, hint }: { title: string; hint?: string }) {
  return (
    <div className="empty-state">
      <Inbox size={30} strokeWidth={1.5} />
      <h3>{title}</h3>
      {hint && <p>{hint}</p>}
    </div>
  )
}

export function Loading() {
  return <div className="loading"><LoaderCircle className="spin" size={22} /></div>
}

export function ProgressBar({ value, indeterminate = false }: {
  value: number
  indeterminate?: boolean
}) {
  return (
    <div className={`progress-track ${indeterminate ? 'indeterminate' : ''}`}>
      <span style={indeterminate ? undefined : { width: `${Math.max(0, Math.min(100, value * 100))}%` }} />
    </div>
  )
}

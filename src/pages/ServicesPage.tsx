import { openUrl } from '@tauri-apps/plugin-opener'
import {
  Check,
  Cloud,
  Edit3,
  ExternalLink,
  Eye,
  EyeOff,
  LoaderCircle,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
} from 'lucide-react'
import { useState, type FormEvent } from 'react'
import { Modal, PageHeader, Toggle } from '../components/ui'
import { useApp } from '../context/AppContext'
import type { ServiceProfile } from '../types'

export default function ServicesPage() {
  const {
    t,
    portable,
    settings,
    services,
    usage,
    saveService,
    deleteService,
    validateService,
    setActiveService,
    notify,
  } = useApp()
  const [editing, setEditing] = useState<ServiceProfile | null>(null)
  const [isNew, setIsNew] = useState(false)
  const [validating, setValidating] = useState<string | null>(null)
  const activeId = settings?.activeServiceId

  async function validate(id: string) {
    setValidating(id)
    try {
      await validateService(id)
    } catch (error) {
      notify(String(error), 'error')
    } finally {
      setValidating(null)
    }
  }

  return (
    <div className="page">
      <PageHeader
        title={t('services.title')}
        subtitle={t('services.subtitle')}
        actions={<button type="button" className="button primary" onClick={() => {
          setIsNew(true)
          setEditing({
            id: crypto.randomUUID(),
            name: 'SoMark',
            baseUrl: 'https://somark.cn/api/v1',
            proxyMode: 'system',
            proxyUrl: null,
            timeoutSecs: 120,
            maxRetries: 3,
            maxFileSizeMb: 200,
            maxPages: 300,
            qps: 4,
            maxConcurrency: 1,
            higherConcurrencyApproved: false,
          })
        }}><Plus size={16} />{t('services.add')}</button>}
      />
      <div className="services-layout">
        <div className="service-list">
          {services.map((service) => (
            <article className={`service-card ${activeId === service.id ? 'selected' : ''}`} key={service.id}>
              <div className="service-card-head">
                <div className="service-icon"><Cloud size={20} /></div>
                <div><h3>{service.name}</h3><p title={service.baseUrl}>{service.baseUrl}</p></div>
                {activeId === service.id && <span className="active-check"><Check size={14} /></span>}
              </div>
              <div className="service-meta"><span>{service.maxFileSizeMb} MB</span><span>QPS {service.qps}</span><span>{service.proxyMode}</span></div>
              <div className="service-card-actions">
                {activeId !== service.id && <button type="button" className="text-button" onClick={() => void setActiveService(service.id)}>{t('services.use')}</button>}
                <button type="button" className="icon-button" onClick={() => { setIsNew(false); setEditing(structuredClone(service)) }}><Edit3 size={16} /></button>
                <button type="button" className="icon-button danger" disabled={services.length <= 1} onClick={async () => {
                  if (!window.confirm(`${t('services.delete')} · ${service.name}?`)) return
                  try { await deleteService(service.id) } catch (error) { notify(String(error), 'error') }
                }}><Trash2 size={16} /></button>
              </div>
            </article>
          ))}
        </div>
        <section className="panel usage-panel">
          <div className="usage-icon"><ShieldCheck size={26} /></div>
          <h2>{t('services.validate')}</h2>
          <p>{t(portable ? 'services.portableKeyHint' : 'services.keyHint')}</p>
          {activeId && usage[activeId] ? (
            <>
              <div className="usage-metrics">
                <div><span>{t('services.paid')}</span><strong>{usage[activeId].remainingPaidPages.toLocaleString()}</strong></div>
                <div><span>{t('services.today')}</span><strong>{usage[activeId].remainingFreePagesToday.toLocaleString()}</strong></div>
                <div><span>{t('services.month')}</span><strong>{usage[activeId].remainingFreePagesThisMonth.toLocaleString()}</strong></div>
              </div>
              <button type="button" className="button ghost" onClick={() => void openUrl(usage[activeId].dashboardUrl)}><ExternalLink size={15} />{t('services.dashboard')}</button>
            </>
          ) : (
            <div className="usage-placeholder">—</div>
          )}
          {activeId && <button type="button" className="button primary" disabled={validating === activeId} onClick={() => void validate(activeId)}>
            {validating === activeId ? <LoaderCircle size={16} className="spin" /> : <RefreshCw size={16} />}{t('services.validate')}
          </button>}
        </section>
      </div>
      {editing && (
        <ServiceEditor
          profile={editing}
          isNew={isNew}
          onClose={() => setEditing(null)}
          onSave={async (profile, key) => {
            await saveService(profile, key)
            setEditing(null)
            notify(t('common.saved'), 'success')
            void validate(profile.id)
          }}
        />
      )}
    </div>
  )
}

function ServiceEditor({ profile, isNew, onClose, onSave }: {
  profile: ServiceProfile
  isNew: boolean
  onClose: () => void
  onSave: (profile: ServiceProfile, key?: string) => Promise<void>
}) {
  const { t, portable, deleteService, notify } = useApp()
  const [draft, setDraft] = useState(profile)
  const [key, setKey] = useState('')
  const [showKey, setShowKey] = useState(false)
  const [busy, setBusy] = useState(false)

  function update<K extends keyof ServiceProfile>(field: K, value: ServiceProfile[K]) {
    setDraft((current) => ({ ...current, [field]: value }))
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (busy || (isNew && !key.trim())) return
    setBusy(true)
    try {
      await onSave(draft, key.trim() || undefined)
    } catch (error) {
      notify(String(error), 'error')
      setBusy(false)
    }
  }

  return (
    <Modal title={t(isNew ? 'services.add' : 'services.edit')} onClose={onClose} wide>
      <form onSubmit={submit}>
        <div className="modal-body form-stack">
          <div className="form-grid two-columns">
            <div className="field"><label>{t('services.name')}</label><input required value={draft.name} onChange={(event) => update('name', event.target.value)} /></div>
            <div className="field"><label>{t('services.baseUrl')}</label><input required type="url" value={draft.baseUrl} onChange={(event) => update('baseUrl', event.target.value)} /></div>
          </div>
          <div className="field">
            <label>{t('services.apiKey')}</label>
            <div className="input-action">
              <input required={isNew} type={showKey ? 'text' : 'password'} autoComplete="off" value={key} placeholder={isNew ? 'sk-…' : '•••••••••••• (unchanged)'} onChange={(event) => setKey(event.target.value)} />
              <button type="button" className="button small" onClick={() => setShowKey(!showKey)}>{showKey ? <EyeOff size={16} /> : <Eye size={16} />}</button>
            </div>
            <small>{t(portable ? 'services.portableKeyHint' : 'services.keyHint')}</small>
          </div>
          <div className="form-grid two-columns">
            <div className="field"><label>{t('services.proxy')}</label><select value={draft.proxyMode} onChange={(event) => update('proxyMode', event.target.value as ServiceProfile['proxyMode'])}><option value="system">{t('services.systemProxy')}</option><option value="direct">{t('services.direct')}</option><option value="custom">{t('services.customProxy')}</option></select></div>
            {draft.proxyMode === 'custom' && <div className="field"><label>{t('services.proxyUrl')}</label><input required type="url" placeholder="http://127.0.0.1:7890" value={draft.proxyUrl ?? ''} onChange={(event) => update('proxyUrl', event.target.value)} /></div>}
          </div>
          <h3>{t('services.limits')}</h3>
          <div className="form-grid three-columns">
            <NumberField label={t('services.maxSize')} value={draft.maxFileSizeMb} min={1} onChange={(value) => update('maxFileSizeMb', value)} />
            <NumberField label={t('services.maxPages')} value={draft.maxPages} min={1} onChange={(value) => update('maxPages', value)} />
            <NumberField label={t('services.qps')} value={draft.qps} min={1} onChange={(value) => update('qps', value)} />
            <NumberField label={t('services.timeout')} value={draft.timeoutSecs} min={5} onChange={(value) => update('timeoutSecs', value)} />
            <NumberField label={t('settings.retries')} value={draft.maxRetries} min={0} onChange={(value) => update('maxRetries', value)} />
            <NumberField label={t('services.concurrency')} value={draft.maxConcurrency} min={1} max={32} onChange={(value) => update('maxConcurrency', value)} />
          </div>
          <Toggle checked={draft.higherConcurrencyApproved} onChange={(value) => update('higherConcurrencyApproved', value)} label={t('services.approved')} />
        </div>
        <div className="modal-footer spread">
          <div>{!isNew && <button type="button" className="button danger-outline" onClick={async () => {
            if (!window.confirm(t('services.delete'))) return
            try { await deleteService(draft.id); onClose() } catch (error) { notify(String(error), 'error') }
          }}>{t('services.delete')}</button>}</div>
          <div className="footer-actions"><button type="button" className="button" onClick={onClose}>{t('common.cancel')}</button><button type="submit" className="button primary" disabled={busy || (isNew && !key.trim())}>{busy ? <LoaderCircle className="spin" size={16} /> : null}{t('services.save')}</button></div>
        </div>
      </form>
    </Modal>
  )
}

function NumberField({ label, value, min, max, onChange }: {
  label: string
  value: number
  min: number
  max?: number
  onChange: (value: number) => void
}) {
  return <div className="field"><label>{label}</label><input type="number" required min={min} max={max} value={value} onChange={(event) => onChange(Number(event.target.value))} /></div>
}

import { open, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { Download, FolderOpen, Save, ShieldCheck } from 'lucide-react'
import { useEffect, useState } from 'react'
import { PageHeader, Toggle } from '../components/ui'
import { useApp } from '../context/AppContext'
import { backend } from '../lib/tauri'
import type { AppSettings } from '../types'

export default function SettingsPage() {
  const { t, settings, version, saveSettings, notify } = useApp()
  const [draft, setDraft] = useState<AppSettings | null>(settings)
  const [busy, setBusy] = useState(false)

  useEffect(() => setDraft(settings), [settings])
  if (!draft) return null

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    setDraft((current) => current ? { ...current, [key]: value } : current)
  }

  async function save() {
    if (!draft) return
    setBusy(true)
    try {
      await saveSettings(draft)
      notify(t('common.saved'), 'success')
    } catch (error) {
      notify(String(error), 'error')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="page settings-page">
      <PageHeader title={t('settings.title')} subtitle={t('settings.subtitle')} actions={<button type="button" className="button primary" disabled={busy} onClick={save}><Save size={16} />{t('settings.save')}</button>} />
      <div className="settings-sections">
        <section className="panel settings-section">
          <h2>{t('settings.appearance')}</h2>
          <div className="settings-row"><label>{t('settings.language')}</label><select value={draft.language} onChange={(event) => update('language', event.target.value as AppSettings['language'])}><option value="zh-CN">简体中文</option><option value="en-US">English</option></select></div>
          <div className="settings-row"><label>{t('settings.theme')}</label><select value={draft.theme} onChange={(event) => update('theme', event.target.value as AppSettings['theme'])}><option value="system">{t('settings.system')}</option><option value="light">{t('settings.light')}</option><option value="dark">{t('settings.dark')}</option></select></div>
          <Toggle checked={draft.notifications} onChange={(value) => update('notifications', value)} label={t('settings.notifications')} />
          <Toggle checked={draft.sound} onChange={(value) => update('sound', value)} label={t('settings.sound')} />
        </section>
        <section className="panel settings-section">
          <h2>{t('settings.queue')}</h2>
          <div className="settings-row"><label>{t('settings.concurrency')}</label><input type="number" min={1} max={32} value={draft.concurrency} onChange={(event) => update('concurrency', Number(event.target.value))} /></div>
          <div className="settings-row"><label>{t('settings.retries')}</label><input type="number" min={0} max={20} value={draft.retryLimit} onChange={(event) => update('retryLimit', Number(event.target.value))} /></div>
          <div className="settings-row"><label>{t('settings.output')}</label><div className="input-action"><input value={draft.defaultOutputRoot} onChange={(event) => update('defaultOutputRoot', event.target.value)} /><button type="button" className="button small" onClick={async () => {
            const selected = await open({ directory: true, multiple: false })
            if (selected) update('defaultOutputRoot', selected)
          }}><FolderOpen size={16} /></button></div></div>
          <Toggle checked={draft.closeToTray} onChange={(value) => update('closeToTray', value)} label={t('settings.closeTray')} />
        </section>
        <section className="panel settings-section">
          <h2>{t('settings.data')}</h2>
          <div className="settings-row"><label>{t('settings.historyDays')}</label><input type="number" min={1} max={3650} value={draft.historyDays} onChange={(event) => update('historyDays', Number(event.target.value))} /></div>
          <div className="settings-row"><label>{t('settings.logDays')}</label><input type="number" min={1} max={365} value={draft.logDays} onChange={(event) => update('logDays', Number(event.target.value))} /></div>
          <div className="settings-row"><label>{t('settings.quota')}</label><input type="number" min={0} value={draft.quotaWarningPages} onChange={(event) => update('quotaWarningPages', Number(event.target.value))} /></div>
          <Toggle checked={draft.blockOnLowQuota} onChange={(value) => update('blockOnLowQuota', value)} label={t('settings.blockQuota')} />
          <div className="privacy-panel"><ShieldCheck size={20} /><p>{t('settings.telemetry')}</p></div>
          <button type="button" className="button diagnostics-button" onClick={async () => {
            const destination = await saveDialog({ defaultPath: `SoMarkClient-CE-${version}-diagnostics.zip` })
            if (!destination) return
            try {
              const path = await backend.exportDiagnostics(destination)
              notify(path, 'success')
            } catch (error) {
              notify(String(error), 'error')
            }
          }}><Download size={15} />Export diagnostics</button>
        </section>
      </div>
      <footer className="about-footer"><div className="brand-symbol small"><span>S</span><i /></div><div><strong>SoMarkClient CE</strong><p>v{version} · Apache-2.0 · Independent SoMark client</p></div></footer>
    </div>
  )
}

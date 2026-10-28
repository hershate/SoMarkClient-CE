import {
  Activity,
  Cloud,
  FilePlus2,
  History,
  ListTodo,
  Settings as SettingsIcon,
  ShieldCheck,
  X,
} from 'lucide-react'
import { NavLink, Outlet } from 'react-router-dom'
import { useApp } from '../context/AppContext'

const navigation = [
  { to: '/', key: 'nav.new', icon: FilePlus2, end: true },
  { to: '/tasks', key: 'nav.tasks', icon: ListTodo },
  { to: '/history', key: 'nav.history', icon: History },
  { to: '/services', key: 'nav.services', icon: Cloud },
  { to: '/settings', key: 'nav.settings', icon: SettingsIcon },
]

export default function Layout() {
  const {
    t,
    version,
    portable,
    settings,
    services,
    tasks,
    toasts,
    dismissToast,
    setActiveService,
    notify,
  } = useApp()
  const active = tasks.filter((task) => ['uploading', 'waiting', 'downloading'].includes(task.status)).length

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-symbol"><span>S</span><i /></div>
          <div>
            <strong>SoMarkClient</strong>
            <span>COMMUNITY EDITION</span>
          </div>
        </div>
        <div className="sidebar-caption">{t('app.tagline')}</div>
        <nav className="main-nav" aria-label="Main navigation">
          {navigation.map(({ to, key, icon: Icon, end }) => (
            <NavLink key={to} to={to} end={end} className={({ isActive }) => isActive ? 'active' : ''}>
              <Icon size={19} strokeWidth={1.8} />
              <span>{t(key)}</span>
              {key === 'nav.tasks' && active > 0 && <b>{active}</b>}
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="privacy-note">
            <ShieldCheck size={17} />
            <span>{t('app.thirdParty')}</span>
          </div>
          <span className="version">v{version}{portable ? ' · Portable' : ''}</span>
        </div>
      </aside>
      <div className="main-column">
        <header className="topbar">
          <div className="connection-indicator"><i /><span>SoMark API</span></div>
          <div className="service-switcher">
            <Cloud size={16} />
            <select
              aria-label="Active service"
              value={settings?.activeServiceId ?? ''}
              onChange={(event) => {
                void setActiveService(event.target.value).catch((error) => notify(String(error), 'error'))
              }}
            >
              {services.map((service) => <option key={service.id} value={service.id}>{service.name}</option>)}
            </select>
          </div>
          <div className="activity-pill"><Activity size={15} /><span>{active}</span></div>
        </header>
        <main className="content"><Outlet /></main>
      </div>
      <div className="toast-stack" aria-live="polite">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast toast-${toast.tone}`}>
            <span>{toast.message}</span>
            <button type="button" onClick={() => dismissToast(toast.id)} aria-label="Dismiss"><X size={15} /></button>
          </div>
        ))}
      </div>
    </div>
  )
}

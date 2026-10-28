import { Navigate, Route, Routes } from 'react-router-dom'
import Layout from './components/Layout'
import { Loading } from './components/ui'
import { AppProvider, useApp } from './context/AppContext'
import HistoryPage from './pages/HistoryPage'
import NewTaskPage from './pages/NewTaskPage'
import ResultPage from './pages/ResultPage'
import ServicesPage from './pages/ServicesPage'
import SettingsPage from './pages/SettingsPage'
import TasksPage from './pages/TasksPage'

export default function App() {
  return (
    <AppProvider>
      <AppRoutes />
    </AppProvider>
  )
}

function AppRoutes() {
  const { ready, fatalError, t } = useApp()
  if (fatalError) {
    return <main className="fatal-screen"><div className="brand-symbol"><span>S</span><i /></div><h1>SoMarkClient CE</h1><p>{fatalError}</p></main>
  }
  if (!ready) return <main className="fatal-screen"><Loading /><p>{t('common.loading')}</p></main>
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<NewTaskPage />} />
        <Route path="tasks" element={<TasksPage />} />
        <Route path="history" element={<HistoryPage />} />
        <Route path="services" element={<ServicesPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="result/:taskId" element={<ResultPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}

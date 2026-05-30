import React from 'react'
import ReactDOM from 'react-dom/client'
import './app/globals.css'
import { QueryProvider } from './components/QueryProvider'
import { I18nProvider } from './i18n'
import { DropGuard } from './components/DropGuard'
import { ConsoleLayout } from './components/ConsoleLayout'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryProvider>
      <I18nProvider>
        <DropGuard />
        <ConsoleLayout />
      </I18nProvider>
    </QueryProvider>
  </React.StrictMode>
)

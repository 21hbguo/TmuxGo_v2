import { createContext, useContext, type ReactNode } from 'react'
import { isTauri } from '@/lib/api-adapter'
import { useWebSocket } from './useWebSocket'
import { useSshTerminal } from './useSshTerminal'

export type Transport = {
  send: (data: any) => any
  isConnected: boolean
  isSocketReady: boolean
  subscribeOutput: (listener: (message: { data: string; sessionName?: string | null }) => void) => () => void
  attach?: (hostId: string, sessionId: string) => Promise<void>
  detach?: (sessionId: string) => Promise<void>
}

const TransportContext = createContext<Transport | null>(null)

export function TransportProvider({ children }: { children: ReactNode }) {
  const ws = useWebSocket()
  const ssh = useSshTerminal()
  const transport = isTauri ? ssh : ws
  return <TransportContext.Provider value={transport}>{children}</TransportContext.Provider>
}

export function useTransport(): Transport {
  const ctx = useContext(TransportContext)
  if (!ctx) throw new Error('useTransport must be used within TransportProvider')
  return ctx
}

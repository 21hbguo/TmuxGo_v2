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

export function useTransport(): Transport {
  const ws = useWebSocket()
  const ssh = useSshTerminal()
  return isTauri ? ssh : ws
}

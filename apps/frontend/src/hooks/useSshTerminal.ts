import { useEffect, useRef, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useConsoleStore } from '@/stores/useConsoleStore'

type OutputMessage = { data: string; sessionName?: string | null }
const outputListeners = new Set<(message: OutputMessage) => void>()

export function useSshTerminal() {
  const updateConnection = useConsoleStore((s) => s.updateConnection)
  const connectionStatus = useConsoleStore((s) => s.connection.status)
  const isConnected = connectionStatus === 'connected'
  const isSocketReady = connectionStatus === 'connected' || connectionStatus === 'attaching'
  const unlistenRef = useRef<UnlistenFn[]>([])
  const attachedRef = useRef(false)
  const currentSessionRef = useRef<string | null>(null)
  const attachRef = useRef<((hostId: string, sessionId: string, cols?: number, rows?: number) => Promise<void>) | null>(null)

  const attach = useCallback(async (hostId: string, sessionId: string, cols?: number, rows?: number) => {
    try {
      currentSessionRef.current = sessionId
      updateConnection({ status: 'attaching' })
      await invoke('attach_terminal', { hostId, sessionId, cols: cols || 120, rows: rows || 36 })
      attachedRef.current = true
      updateConnection({ status: 'connected' })
      const sessionName = sessionId.replace('session-', '')
      window.dispatchEvent(new CustomEvent('tmux-attached', { detail: { sessionName, sessionId, cols, rows } }))
    } catch (err) {
      console.error('Failed to attach terminal:', err)
      if (currentSessionRef.current === sessionId) {
        attachedRef.current = false
        updateConnection({ status: 'disconnected' })
      }
    }
  }, [updateConnection])
  attachRef.current = attach

  const send = useCallback(async (data: any) => {
    if (data.type === 'attach') {
      const state = useConsoleStore.getState()
      const hostId = state.activeHostId
      const sessionId = state.activeSessionId
      if (hostId && sessionId && attachRef.current) {
        await attachRef.current(hostId, sessionId, data.cols, data.rows)
      }
      return true
    }
    if (data.type === 'input') {
      const sessionId = useConsoleStore.getState().activeSessionId
      if (!sessionId) return false
      await invoke('send_terminal_input', { sessionId, data: data.data })
      return true
    }
    if (data.type === 'resize') {
      const sessionId = useConsoleStore.getState().activeSessionId
      if (!sessionId) return false
      await invoke('resize_terminal', { sessionId, cols: data.cols, rows: data.rows })
      return true
    }
    return false
  }, [])

  const subscribeOutput = useCallback((listener: (message: OutputMessage) => void) => {
    outputListeners.add(listener)
    return () => outputListeners.delete(listener)
  }, [])

  const detach = useCallback(async (sessionId: string) => {
    try {
      await invoke('detach_terminal', { sessionId })
    } catch {}
    attachedRef.current = false
    updateConnection({ status: 'disconnected' })
  }, [updateConnection])

  useEffect(() => {
    const setup = async () => {
      const unlistenOutput = await listen<{ data: string; sessionId?: string }>('terminal-output', (event) => {
        if (event.payload.sessionId && event.payload.sessionId !== currentSessionRef.current) return
        Array.from(outputListeners).forEach((listener) => listener(event.payload))
      })
      const unlistenClosed = await listen<{ sessionId?: string }>('terminal-closed', (event) => {
        if (event.payload.sessionId && event.payload.sessionId !== currentSessionRef.current) return
        attachedRef.current = false
        currentSessionRef.current = null
        updateConnection({ status: 'disconnected' })
      })
      const unlistenError = await listen<{ sessionId?: string }>('terminal-error', (event) => {
        if (event.payload.sessionId && event.payload.sessionId !== currentSessionRef.current) return
        console.error('Terminal error:', event.payload)
        attachedRef.current = false
        currentSessionRef.current = null
        updateConnection({ status: 'disconnected' })
      })
      unlistenRef.current = [unlistenOutput, unlistenClosed, unlistenError]
    }
    void setup()
    return () => {
      unlistenRef.current.forEach((fn) => fn())
      unlistenRef.current = []
    }
  }, [updateConnection])

  return { send, isConnected, isSocketReady, subscribeOutput, attach, detach }
}

'use client'
import { useState, useCallback } from 'react'
import { useHosts } from '@/hooks/useApi'
import { currentApi } from '@/lib/api-adapter'
import { isTauri } from '@/lib/api-adapter'
import { useConsoleStore } from '@/stores/useConsoleStore'
import { useQueryClient } from '@tanstack/react-query'

interface HostForm {
  name: string
  host: string
  port: number
  username: string
  authType: 'password' | 'key' | 'agent'
  password: string
  keyPath: string
  keyPassphrase: string
  group: string
}

const emptyForm: HostForm = {
  name: '',
  host: '',
  port: 22,
  username: 'root',
  authType: 'password',
  password: '',
  keyPath: '',
  keyPassphrase: '',
  group: '',
}

export function ConnectionManager({ onClose }: { onClose: () => void }) {
  const { data: hostsData = [] } = useHosts()
  const setActiveHost = useConsoleStore((s) => s.setActiveHost)
  const queryClient = useQueryClient()
  const [editingId, setEditingId] = useState<string | null>(null)
  const [form, setForm] = useState<HostForm>(emptyForm)
  const [showForm, setShowForm] = useState(false)
  const [connecting, setConnecting] = useState<string | null>(null)
  const [error, setError] = useState('')

  const resetForm = useCallback(() => {
    setForm(emptyForm)
    setEditingId(null)
    setShowForm(false)
    setError('')
  }, [])

  const handleEdit = useCallback((host: any) => {
    setForm({
      name: host.name || '',
      host: host.host || host.address || '',
      port: host.port || 22,
      username: host.username || 'root',
      authType: host.authType || host.auth_type || 'password',
      password: '',
      keyPath: host.keyPath || host.key_path || '',
      keyPassphrase: '',
      group: host.group || '',
    })
    setEditingId(host.id)
    setShowForm(true)
  }, [])

  const handleSave = useCallback(async () => {
    if (!form.name.trim() || !form.host.trim() || !form.username.trim()) {
      setError('Name, host, and username are required')
      return
    }
    try {
      const payload = {
        id: editingId || `host-${Date.now()}`,
        name: form.name.trim(),
        host: form.host.trim(),
        port: form.port,
        username: form.username.trim(),
        auth_type: form.authType,
        password: form.authType === 'password' ? form.password : undefined,
        key_path: form.authType === 'key' ? form.keyPath : undefined,
        key_passphrase: form.authType === 'key' ? form.keyPassphrase || undefined : undefined,
        group: form.group.trim() || undefined,
      }
      if (editingId) {
        await currentApi.hosts.update?.(editingId, payload)
      } else {
        await currentApi.hosts.create?.(payload)
      }
      queryClient.invalidateQueries({ queryKey: ['hosts'] })
      resetForm()
    } catch (err: any) {
      setError(err.message || 'Failed to save host')
    }
  }, [form, editingId, queryClient, resetForm])

  const handleDelete = useCallback(async (id: string) => {
    if (!window.confirm('Delete this host?')) return
    try {
      await currentApi.hosts.delete?.(id)
      queryClient.invalidateQueries({ queryKey: ['hosts'] })
    } catch (err: any) {
      setError(err.message || 'Failed to delete host')
    }
  }, [queryClient])

  const handleConnect = useCallback(async (host: any) => {
    setConnecting(host.id)
    setError('')
    try {
      await currentApi.hosts.connect?.(host.id)
      setActiveHost(host.id)
      onClose()
    } catch (err: any) {
      setError(err.message || 'Connection failed')
    } finally {
      setConnecting(null)
    }
  }, [setActiveHost, onClose])

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="w-full max-w-2xl max-h-[80vh] rounded-xl border border-[var(--line)] bg-bg-1 shadow-2xl flex flex-col">
        <div className="flex items-center justify-between border-b border-[var(--line)] px-5 py-4">
          <h2 className="text-lg font-semibold text-text-1">SSH Connections</h2>
          <div className="flex items-center gap-2">
            <button
              onClick={() => { setForm(emptyForm); setEditingId(null); setShowForm(true) }}
              className="rounded-lg bg-accent/20 px-3 py-1.5 text-sm text-accent hover:bg-accent/30"
            >
              + Add Host
            </button>
            <button onClick={onClose} className="rounded-lg px-2 py-1 text-text-3 hover:text-text-1">✕</button>
          </div>
        </div>

        {error && (
          <div className="mx-5 mt-3 rounded-lg bg-red-500/10 px-4 py-2 text-sm text-red-400">{error}</div>
        )}

        {showForm && (
          <div className="border-b border-[var(--line)] p-5">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="mb-1 block text-xs text-text-3">Name</label>
                <input
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                  placeholder="My Server"
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-text-3">Host</label>
                <input
                  value={form.host}
                  onChange={(e) => setForm({ ...form, host: e.target.value })}
                  className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                  placeholder="192.168.1.100"
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-text-3">Port</label>
                <input
                  type="number"
                  value={form.port}
                  onChange={(e) => setForm({ ...form, port: parseInt(e.target.value) || 22 })}
                  className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-text-3">Username</label>
                <input
                  value={form.username}
                  onChange={(e) => setForm({ ...form, username: e.target.value })}
                  className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                />
              </div>
              <div>
                <label className="mb-1 block text-xs text-text-3">Auth Type</label>
                <select
                  value={form.authType}
                  onChange={(e) => setForm({ ...form, authType: e.target.value as any })}
                  className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                >
                  <option value="password">Password</option>
                  <option value="key">SSH Key</option>
                  <option value="agent">SSH Agent</option>
                </select>
              </div>
              {form.authType === 'password' && (
                <div>
                  <label className="mb-1 block text-xs text-text-3">Password</label>
                  <input
                    type="password"
                    value={form.password}
                    onChange={(e) => setForm({ ...form, password: e.target.value })}
                    className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                  />
                </div>
              )}
              {form.authType === 'key' && (
                <>
                  <div>
                    <label className="mb-1 block text-xs text-text-3">Key Path</label>
                    <input
                      value={form.keyPath}
                      onChange={(e) => setForm({ ...form, keyPath: e.target.value })}
                      className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                      placeholder="~/.ssh/id_rsa"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs text-text-3">Passphrase</label>
                    <input
                      type="password"
                      value={form.keyPassphrase}
                      onChange={(e) => setForm({ ...form, keyPassphrase: e.target.value })}
                      className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                    />
                  </div>
                </>
              )}
              <div>
                <label className="mb-1 block text-xs text-text-3">Group</label>
                <input
                  value={form.group}
                  onChange={(e) => setForm({ ...form, group: e.target.value })}
                  className="w-full rounded-lg border border-[var(--line)] bg-bg-0 px-3 py-2 text-sm text-text-1 outline-none focus:border-accent"
                  placeholder="Production"
                />
              </div>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={resetForm} className="rounded-lg px-4 py-2 text-sm text-text-3 hover:text-text-1">Cancel</button>
              <button onClick={handleSave} className="rounded-lg bg-accent/20 px-4 py-2 text-sm text-accent hover:bg-accent/30">
                {editingId ? 'Update' : 'Add'}
              </button>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto p-5">
          {hostsData.length === 0 ? (
            <div className="py-12 text-center text-text-3">
              No hosts configured. Click "+ Add Host" to get started.
            </div>
          ) : (
            <div className="space-y-2">
              {hostsData.map((host: any) => (
                <div
                  key={host.id}
                  className="flex items-center justify-between rounded-lg border border-[var(--line)] bg-bg-0 px-4 py-3"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium text-text-1">{host.name}</span>
                      {host.group && (
                        <span className="rounded bg-bg-2 px-1.5 py-0.5 text-[10px] text-text-3">{host.group}</span>
                      )}
                    </div>
                    <div className="mt-0.5 text-xs text-text-3">
                      {host.host || host.address}:{host.port || 22} · {host.username || 'root'}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleConnect(host)}
                      disabled={connecting === host.id}
                      className={`rounded-lg px-3 py-1.5 text-xs ${
                        connecting === host.id
                          ? 'bg-bg-2 text-text-3'
                          : 'bg-accent/20 text-accent hover:bg-accent/30'
                      }`}
                    >
                      {connecting === host.id ? 'Connecting...' : 'Connect'}
                    </button>
                    <button
                      onClick={() => handleEdit(host)}
                      className="rounded-lg px-2 py-1.5 text-xs text-text-3 hover:text-text-1"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(host.id)}
                      className="rounded-lg px-2 py-1.5 text-xs text-text-3 hover:text-red-400"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

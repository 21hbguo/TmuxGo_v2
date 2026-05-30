import type { CustomShortcut, FavoriteDirectory, FileContentMatch, FileContentResponse, FileItem, FileListResponse, FilePreviewResponse, FileRoot, FileUploadTarget, RemotePreferences, UploadJobResult } from '@/types'
import { api, type SystemInfoResponse } from './api'
import { tauriApi } from './tauri-commands'

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const tauriAdapter = {
  snapshot: {
    get: (hostId: string, sessionId: string) => tauriApi.snapshot.get(hostId, sessionId),
  },
  hosts: {
    list: () => tauriApi.hosts.list(),
    get: (id: string) => tauriApi.hosts.get(id),
    create: (host: any) => tauriApi.hosts.create(host),
    update: (id: string, host: any) => tauriApi.hosts.update(id, host),
    delete: (id: string) => tauriApi.hosts.delete(id),
    connect: (id: string) => tauriApi.hosts.connect(id),
  },
  sessions: {
    list: (hostId: string) => tauriApi.sessions.list(hostId),
    create: (hostId: string, name: string, layout?: { windows: { name: string; panes: { command?: string }[] }[] }) =>
      tauriApi.sessions.create(hostId, name, layout),
    rename: (hostId: string, sessionId: string, name: string) =>
      tauriApi.sessions.rename(hostId, sessionId, name),
    delete: (hostId: string, sessionId: string) =>
      tauriApi.sessions.delete(hostId, sessionId),
  },
  windows: {
    list: (hostId: string, sessionId: string) => tauriApi.windows.list(hostId, sessionId),
    create: (hostId: string, sessionId: string, name: string) =>
      tauriApi.windows.create(hostId, sessionId, name),
    select: (hostId: string, sessionId: string, windowId: string) =>
      tauriApi.windows.select(hostId, sessionId, windowId),
    rename: (hostId: string, sessionId: string, windowId: string, name: string) =>
      tauriApi.windows.rename(hostId, sessionId, windowId, name),
    move: (hostId: string, sessionId: string, orderedWindowIds: string[]) =>
      tauriApi.windows.move(hostId, sessionId, orderedWindowIds),
    kill: (hostId: string, sessionId: string, windowId: string) =>
      tauriApi.windows.kill(hostId, sessionId, windowId),
  },
  panes: {
    list: (windowId: string) => tauriApi.panes.list(windowId),
    listBySession: (hostId: string, sessionId: string) =>
      tauriApi.panes.listBySession(hostId, sessionId),
    output: (paneId: string) => tauriApi.panes.output(paneId),
    create: (windowId: string, direction: 'horizontal' | 'vertical') =>
      tauriApi.panes.create(windowId, direction),
    zoom: (paneId?: string) => tauriApi.panes.zoom(paneId),
    select: (paneId: string) => tauriApi.panes.select(paneId),
    split: (paneId: string, direction: 'horizontal' | 'vertical') =>
      tauriApi.panes.split(paneId, direction),
    zoomByPane: (paneId: string) => tauriApi.panes.zoomByPane(paneId),
    kill: (paneId?: string) => tauriApi.panes.kill(paneId),
  },
  system: {
    info: () => tauriApi.system.info() as Promise<SystemInfoResponse>,
  },
  files: {
    roots: () => tauriApi.files.roots() as Promise<FileRoot[]>,
    list: (root: string, path = '') =>
      tauriApi.files.list(root, root, path) as Promise<FileListResponse>,
    preview: (root: string, path: string, line = 1) =>
      tauriApi.files.preview(root, root, path, line) as Promise<FilePreviewResponse>,
    content: (root: string, path: string) =>
      tauriApi.files.content(root, root, path) as Promise<FileContentResponse>,
    saveContent: (root: string, path: string, content: string, modifiedAt?: string) =>
      tauriApi.files.saveContent(root, root, path, content, modifiedAt),
    searchName: (root: string, q: string, basePath = '') =>
      tauriApi.files.searchName(root, root, q, basePath) as Promise<FileItem[]>,
    searchContent: (root: string, q: string, basePath = '') =>
      tauriApi.files.searchContent(root, root, q, basePath) as Promise<FileContentMatch[]>,
    defaultUploadTarget: (_paneId?: string) =>
      tauriApi.files.defaultUploadTarget('') as Promise<FileUploadTarget>,
    upload: async (body: FormData, _onProgress?: (loadedBytes: number, totalBytes: number) => void) => {
      const file = body.get('file') as File | null
      const rootId = (body.get('rootId') as string) || ''
      const targetPath = (body.get('path') as string) || ''
      if (!file) throw new Error('No file in FormData')
      const buffer = await file.arrayBuffer()
      const fileData = Array.from(new Uint8Array(buffer))
      return tauriApi.files.upload(rootId, rootId, targetPath, file.name, fileData)
    },
  },
  preferences: {
    get: (profile = 'default') => tauriApi.preferences.get(profile) as Promise<RemotePreferences>,
    update: (payload: { customShortcuts?: CustomShortcut[]; customShortcutsUpdatedAt?: string; favoriteDirectories?: FavoriteDirectory[]; favoriteDirectoriesUpdatedAt?: string }, profile = 'default') =>
      tauriApi.preferences.update(payload, profile) as Promise<RemotePreferences>,
  },
}

export type ApiInterface = typeof api
export const currentApi: ApiInterface = isTauri ? tauriAdapter as unknown as typeof api : api

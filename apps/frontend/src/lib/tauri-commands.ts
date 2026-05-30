import { invoke } from '@tauri-apps/api/core'
import type { CustomShortcut, FavoriteDirectory, FileContentMatch, FileContentResponse, FileItem, FileListResponse, FilePreviewResponse, FileRoot, FileUploadTarget, RemotePreferences, UploadJobResult } from '@/types'
import type { SystemInfoResponse } from './api'

export const tauriApi = {
  snapshot: {
    get: (hostId: string, sessionId: string) =>
      invoke<{ sessionId: string; sessionName: string; windows: any[]; panes: any[]; activeWindowId: string | null; activePaneId: string | null }>(
        'get_snapshot', { hostId, sessionId }
      ),
  },
  hosts: {
    list: () => invoke<any[]>('list_hosts'),
    get: (id: string) => invoke<any>('get_host', { id }),
    create: (host: any) => invoke<any>('create_host', { host }),
    update: (id: string, host: any) => invoke<any>('update_host', { id, host }),
    delete: (id: string) => invoke<void>('delete_host', { id }),
    connect: (id: string) => invoke<void>('connect_host', { id }),
  },
  sessions: {
    list: (hostId: string) => invoke<any[]>('list_sessions', { hostId }),
    create: (hostId: string, name: string, layout?: { windows: { name: string; panes: { command?: string }[] }[] }) =>
      invoke<any>('create_session', { hostId, name, layout }),
    rename: (hostId: string, sessionId: string, name: string) =>
      invoke<any>('rename_session', { hostId, sessionId, name }),
    delete: (hostId: string, sessionId: string) =>
      invoke<any>('delete_session', { hostId, sessionId }),
  },
  windows: {
    list: (hostId: string, sessionId: string) =>
      invoke<any[]>('list_windows', { hostId, sessionId }),
    create: (hostId: string, sessionId: string, name: string) =>
      invoke<any>('create_window', { hostId, sessionId, name }),
    select: (hostId: string, sessionId: string, windowId: string) =>
      invoke<any>('select_window', { hostId, sessionId, windowId }),
    rename: (hostId: string, sessionId: string, windowId: string, name: string) =>
      invoke<any>('rename_window', { hostId, sessionId, windowId, name }),
    move: (hostId: string, sessionId: string, orderedWindowIds: string[]) =>
      invoke<any>('move_windows', { hostId, sessionId, orderedWindowIds }),
    kill: (hostId: string, sessionId: string, windowId: string) =>
      invoke<any>('kill_window', { hostId, sessionId, windowId }),
  },
  panes: {
    list: (windowId: string) => invoke<any[]>('list_panes', { windowId }),
    listBySession: (hostId: string, sessionId: string) =>
      invoke<any[]>('list_session_panes', { hostId, sessionId }),
    output: (paneId: string) =>
      invoke<{ paneId: string; tmuxPaneId?: string; data: string }>('get_pane_output', { paneId }),
    create: (windowId: string, direction: 'horizontal' | 'vertical') =>
      invoke<any>('split_pane', { windowId, direction }),
    zoom: (paneId?: string) =>
      invoke<any>('zoom_pane', { paneId }),
    select: (paneId: string) =>
      invoke<any>('select_pane', { paneId }),
    split: (paneId: string, direction: 'horizontal' | 'vertical') =>
      invoke<any>('split_pane', { paneId, direction }),
    zoomByPane: (paneId: string) =>
      invoke<any>('zoom_pane', { paneId }),
    kill: (paneId?: string) =>
      invoke<any>('kill_pane', { paneId }),
  },
  system: {
    info: (hostId?: string) => invoke<SystemInfoResponse>('get_system_info', { hostId }),
  },
  files: {
    roots: (hostId?: string) => invoke<FileRoot[]>('list_file_roots', { hostId }),
    list: (hostId: string, root: string, path = '') =>
      invoke<FileListResponse>('list_files', { hostId, root, path }),
    preview: (hostId: string, root: string, path: string, line = 1) =>
      invoke<FilePreviewResponse>('read_file_preview', { hostId, root, path, line }),
    content: (hostId: string, root: string, path: string) =>
      invoke<FileContentResponse>('read_file_content', { hostId, root, path }),
    saveContent: (hostId: string, root: string, path: string, content: string, modifiedAt?: string) =>
      invoke<{ ok: true; content: string; modifiedAt: string; size: number }>('save_file_content', { hostId, root, path, content, modifiedAt }),
    searchName: (hostId: string, root: string, q: string, basePath = '') =>
      invoke<FileItem[]>('search_files_by_name', { hostId, root, q, basePath }),
    searchContent: (hostId: string, root: string, q: string, basePath = '') =>
      invoke<FileContentMatch[]>('search_files_by_content', { hostId, root, q, basePath }),
    defaultUploadTarget: (hostId: string, paneId?: string) =>
      invoke<FileUploadTarget>('get_default_upload_target', { hostId, paneId }),
    upload: async (hostId: string, rootId: string, targetPath: string, fileName: string, fileData: number[]) =>
      invoke<UploadJobResult>('upload_file', { hostId, rootId, targetPath, fileName, fileData }),
  },
  preferences: {
    get: (profile = 'default') =>
      invoke<RemotePreferences>('get_preferences', { profile }),
    update: (payload: { customShortcuts?: CustomShortcut[]; customShortcutsUpdatedAt?: string; favoriteDirectories?: FavoriteDirectory[]; favoriteDirectoriesUpdatedAt?: string }, profile = 'default') =>
      invoke<RemotePreferences>('update_preferences', { profile, payload }),
  },
}

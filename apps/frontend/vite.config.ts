import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tsconfigPaths from 'vite-tsconfig-paths'
import { fileURLToPath } from 'url'
import path from 'path'

const root = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig({
  root,
  plugins: [react(), tsconfigPaths()],
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: path.resolve(root, '../../dist'),
    emptyOutDir: true,
    target: 'esnext',
  },
})

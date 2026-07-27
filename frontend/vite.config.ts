import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import svgr from 'vite-plugin-svgr';

export default defineConfig({
  base: './',
  plugins: [react(), tailwindcss(), svgr()],
  server: {
    port: 5173,
    proxy: {
      // Proxy /api → CULI backend (port 3111)
      '/api': {
        target: 'http://localhost:3111',
        changeOrigin: true,
      },
      // Proxy /v1 → CulirouterAPI (port 4000)
      '/v1': {
        target: 'http://localhost:4000',
        changeOrigin: true,
      },
      '/health': {
        target: 'http://localhost:4000',
        changeOrigin: true,
      },
      '/stats': {
        target: 'http://localhost:4000',
        changeOrigin: true,
      },
    },
  },
});

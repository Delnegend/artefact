import { fileURLToPath, URL } from 'node:url'

import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
	plugins: [
		vue(),
		tailwindcss(),
		VitePWA({
			registerType: 'autoUpdate',
			manifest: {
				name: 'Artefact',
				short_name: 'Artefact',
				theme_color: '#020817',
				background_color: '#020817',
				icons: [
					{
						src: '/pwa-64.png',
						sizes: '64x64',
						type: 'image/png'
					},
					{
						src: '/pwa-192.png',
						sizes: '192x192',
						type: 'image/png'
					},
					{
						src: '/pwa-512.png',
						sizes: '512x512',
						type: 'image/png'
					},
					{
						src: '/maskable-512.png',
						sizes: '512x512',
						type: 'image/png',
						purpose: 'maskable'
					}
				],
				display: 'standalone'
			},
			workbox: {
				globPatterns: ['**/*.{js,css,html,svg,png,ico,woff2,wasm}'],
				cleanupOutdatedCaches: true,
				clientsClaim: true
			},
			devOptions: {
				enabled: true,
				navigateFallback: 'index.html',
				suppressWarnings: true,
				/* when using generateSW the PWA plugin will switch to classic */
				type: 'module'
			}
		})
	],
	resolve: {
		alias: {
			'~': fileURLToPath(new URL('./src', import.meta.url)),
			'artefact-wasm': fileURLToPath(
				new URL(
					'./src/utils/artefact-wasm/artefact_wasm.js',
					import.meta.url
				)
			)
		}
	},
	build: { target: 'esnext' },
	worker: {
		format: 'es',
		rollupOptions: { output: { format: 'es' } }
	}
})

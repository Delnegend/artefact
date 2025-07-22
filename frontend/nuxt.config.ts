// https://nuxt.com/docs/api/configuration/nuxt-config
import tailwindcss from '@tailwindcss/vite'
import { defineNuxtConfig } from 'nuxt/config'
import { VitePWA } from 'vite-plugin-pwa'
import wasm from 'vite-plugin-wasm'

export default defineNuxtConfig({
	modules: ['@vite-pwa/nuxt'],
	ssr: false,
	components: {
		dirs: []
	},
	imports: {
		scan: false,
		autoImport: false
	},
	app: {
		head: {
			title: 'Artefact',
			meta: [
				{ charset: 'utf-8' },
				{
					name: 'viewport',
					content:
						'width=device-width, initial-scale=1, maximum-scale=1, user-scalable=0'
				},
				{
					name: 'keywords',
					content:
						'rust, jpeg, image, artifact, remove, artifacts, jpeg artifacts, jpeg artifact, png, jpeg to png'
				},
				{
					name: 'description',
					content:
						'Remove JPEG artifacts from images, right in your browser.'
				}
			]
		}
	},
	css: ['~/assets/css/main.css'],
	srcDir: 'src',
	compatibilityDate: '2024-12-29',
	vite: {
		build: { target: 'esnext' },
		worker: {
			format: 'es',
			rollupOptions: { output: { format: 'es' } }
		},
		plugins: [
			tailwindcss(),
			wasm(),
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
					globPatterns: ['**/*.{js,css,html,svg,png,svg,ico}'],
					cleanupOutdatedCaches: true,
					clientsClaim: true
				},
				injectManifest: {
					globPatterns: ['**/*.{js,css,html,svg,png,svg,ico}']
				},
				devOptions: {
					enabled: true,
					navigateFallback: 'index.html',
					suppressWarnings: true,
					/* when using generateSW the PWA plugin will switch to classic */
					type: 'module'
				}
			})
		]
	},
	pwa: {
		client: {
			installPrompt: true
		}
	}
})

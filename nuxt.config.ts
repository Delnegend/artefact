// https://nuxt.com/docs/api/configuration/nuxt-config
import { defineNuxtConfig } from "nuxt/config";
import { VitePWA } from "vite-plugin-pwa";
import wasm from "vite-plugin-wasm";

export default defineNuxtConfig({
	modules: ["@nuxt/eslint", "@vueuse/nuxt", "@nuxtjs/tailwindcss", "@nuxtjs/color-mode", "shadcn-nuxt"],
	ssr: false,
	components: {
		dirs: [],
	},
	imports: {
		scan: false,
	},
	app: {
		head: {
			style: ["html { background-color: black; }"],
			title: "Artefact",
			meta: [
				{ charset: "utf-8" },
				{ name: "viewport", content: "width=device-width, initial-scale=1" },
			],
		},
	},
	css: ["~/assets/css/main.css"],
	srcDir: "src",
	compatibilityDate: "2024-12-29",
	vite: {
		build: { target: "esnext" },
		worker: {
			format: "es",
			rollupOptions: { output: { format: "es" } },
		},
		plugins: [
			wasm(),
			VitePWA({
				registerType: "autoUpdate",
				manifest: {
					name: "Artefact",
					short_name: "Artefact",
					theme_color: "#000000",
					background_color: "#000000",
				},
				devOptions: {
					enabled: true,
				},
			}),
		],
	},
	postcss: {
		plugins: {
			tailwindcss: {},
			autoprefixer: {},
		},
	},
	shadcn: {
		componentDir: "./src/components/ui",
	},
});

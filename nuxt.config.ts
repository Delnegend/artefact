// https://nuxt.com/docs/api/configuration/nuxt-config
import { defineNuxtConfig } from "nuxt/config";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";

export default defineNuxtConfig({
	modules: ["@nuxt/eslint", "@vueuse/nuxt", "@nuxtjs/tailwindcss", "@nuxtjs/color-mode", "shadcn-nuxt"],
	srcDir: "src",
	ssr: false,
	css: ["~/assets/css/main.css"],
	app: {
		head: {
			title: "Artefact",
			meta: [
				{ charset: "utf-8" },
				{ name: "viewport", content: "width=device-width, initial-scale=1" },
			],
			script: [
				{ src: "/icat.js", fetchpriority: "high" },
			],
		},
	},
	vite: {
		plugins: [wasm(), topLevelAwait()],
	},
	experimental: {
		typedPages: true,
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
	compatibilityDate: "2024-12-29",
});

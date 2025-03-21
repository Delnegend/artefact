import hagemanto from "eslint-plugin-hagemanto";
import pluginVue from "eslint-plugin-vue";

export default [
	{ files: ["**/*.{ts,vue}"] },
	{ ignores: ["artefact-wasm/**/*", "src/utils/artefact-wasm/**/*", "node_modules/**/*", "src/dev-dist/**/*", ".nuxt/**/*", "*.config.*"] },

	...hagemanto({
		enableJsx: false,
		vueConfig: pluginVue.configs["flat/recommended"],
	}),
];

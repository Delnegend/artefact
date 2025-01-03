import hagemanto from "eslint-plugin-hagemanto";
import globals from "globals";

import withNuxt from "./.nuxt/eslint.config.mjs";

export default withNuxt(
	{
		rules: {
			"vue/multi-word-component-names": "off",
		},
	},
).prepend(
	{ files: ["**/*.{ts,vue}"] },
	{ ignores: ["artefact-wasm/**/*", "src/composables/artefact-wasm/**/*"] },
	// eslint-disable-next-line @typescript-eslint/no-unsafe-argument, @typescript-eslint/no-unsafe-call
	...hagemanto({
		enableJsx: false,
		enableTailwind: false,
		enableTs: true,
		sortImports: true,
		styler: "stylistic",
	}),
	{
		languageOptions: {
			globals: globals.browser, parserOptions: {
				project: true, parser: "@typescript-eslint/parser", extraFileExtensions: [".vue"],
			},
		},
	},
);

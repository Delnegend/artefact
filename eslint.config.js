import hagemanto from "eslint-plugin-hagemanto";
import pluginVue from "eslint-plugin-vue";

export default [
	{ files: ["**/*.{ts,vue}"] },
	{ ignores: ["artefact-wasm/**/*", "src/utils/artefact-wasm/**/*", "node_modules/**/*", "src/dev-dist/**/*", ".nuxt/**/*"] },

	// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	...hagemanto({
		enableJsx: false,
		extraFileExtensions: [".vue"],
	}),
	...pluginVue.configs["flat/recommended"],

	{
		rules: {
			"no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
			"tailwindcss/no-custom-classname": "off",
			"vue/html-indent": ["error", "tab"],
			"vue/multi-word-component-names": "off",
			"vue/html-closing-bracket-newline": [
				"error",
				{
					singleline: "never",
					multiline: "never",
					selfClosingTag: {
						singleline: "never",
						multiline: "never",
					},
				},
			],
		},
	},
];

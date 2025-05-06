import hagemanto from "eslint-plugin-hagemanto";
import pluginVue from "eslint-plugin-vue";

/** @type { import("eslint").Linter.Config[] } */
export default [
	{
		files: ["**/*.{ts,vue}"],
	}, {
		ignores: [
			"artefact-wasm/**/*",
			"src/utils/artefact-wasm/**/*",
			"node_modules/**/*",
			"src/dev-dist/**/*",
			".nuxt/**/*",
		],
		// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	}, ...hagemanto({
		enableJsx: false,
		vueConfig: pluginVue.configs["flat/recommended"],
	}), {
		languageOptions: {
			globals: {
				document: true,
				window: true,
				module: true,
				require: true,
			},
		},
	}, {
		rules: {
			"no-unused-vars": 0,
		},
	}, {
		files: ["src/components/ui/dropdown-menu/*"],
		rules: {
			"@typescript-eslint/no-unsafe-assignment": 0,
		},
	},
];

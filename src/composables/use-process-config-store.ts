import { clamp } from "@vueuse/core";
import { defineStore } from "pinia";
import { computed, ref, watchEffect } from "vue";
import { OutputImgFormat, type ProcessingConfig } from "~/utils/types";

export const useProcessConfigStore = defineStore("processing-config", () => {
	const __DEFAULT_OUTPUT_FORMAT = OutputImgFormat.PNG;
	const __DEFAULT_ITERATIONS = 50;
	const __DEFAULT_WEIGHT = 0.3;
	const __DEFAULT_P_WEIGHT = 0.001;
	const __DEFAULT_SEPARATE_COMPONENTS = false;

	const outputFormat = ref<OutputImgFormat>(__DEFAULT_OUTPUT_FORMAT);
	const iterations = ref<number>(__DEFAULT_ITERATIONS);
	const weight = ref<number>(__DEFAULT_WEIGHT);
	const pWeight = ref<number>(__DEFAULT_P_WEIGHT);
	const separateComponents = ref<boolean>(__DEFAULT_SEPARATE_COMPONENTS);

	try {
		const config = localStorage.getItem("processing-config");
		if (config) {
			const parsedConfig = JSON.parse(config) as ProcessingConfig;
			outputFormat.value = parsedConfig.outputFormat;
			iterations.value = parsedConfig.iterations;
			weight.value = parsedConfig.weight;
			pWeight.value = parsedConfig.pWeight;
			separateComponents.value = parsedConfig.separateComponents;
		} else {
			localStorage.setItem(
				"processing-config",
				JSON.stringify({
					outputFormat: outputFormat.value,
					iterations: iterations.value,
					weight: weight.value,
					pWeight: pWeight.value,
					separateComponents: separateComponents.value,
				}),
			);
		}
	} catch {
		localStorage.removeItem("processing-config");
	}

	const isDefault = ref(true);
	watchEffect(() => {
		isDefault.value = outputFormat.value === __DEFAULT_OUTPUT_FORMAT
			&& iterations.value === __DEFAULT_ITERATIONS
			&& weight.value === __DEFAULT_WEIGHT
			&& pWeight.value === __DEFAULT_P_WEIGHT
			&& separateComponents.value === __DEFAULT_SEPARATE_COMPONENTS;
	});

	watchEffect(() => {
		localStorage.setItem(
			"processing-config",
			JSON.stringify({
				outputFormat: outputFormat.value,
				iterations: iterations.value,
				weight: weight.value,
				pWeight: pWeight.value,
				separateComponents: separateComponents.value,
			}),
		);
	});

	function resetDefaultAll(): void {
		outputFormat.value = __DEFAULT_OUTPUT_FORMAT;
		iterations.value = __DEFAULT_ITERATIONS;
		weight.value = __DEFAULT_WEIGHT;
		pWeight.value = __DEFAULT_P_WEIGHT;
		separateComponents.value = __DEFAULT_SEPARATE_COMPONENTS;
	}

	return {
		outputFormat,
		iterations,
		weight,
		pWeight,
		separateComponents,
		isDefault,
		resetDefaultAll,
		handleOutputFormatChange: (format: OutputImgFormat): void => {
			outputFormat.value = format;
		},
		ensureInterationsValid: (): void => {
			iterations.value = clamp(iterations.value, 1, 1000);
		},
		ensureWeightValid: (): void => {
			weight.value = clamp(weight.value, 0, 1);
		},
		ensurePWeightValid: (): void => {
			pWeight.value = clamp(pWeight.value, 0, 1);
		},
		allConfig: computed(
			(): ProcessingConfig => ({
				outputFormat: outputFormat.value,
				iterations: iterations.value,
				weight: weight.value,
				pWeight: pWeight.value,
				separateComponents: separateComponents.value,
			}),
		),
	};
});

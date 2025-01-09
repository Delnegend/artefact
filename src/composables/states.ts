import { openDB } from "idb";
import { defineStore } from "pinia";
import { type Ref, ref, watchEffect } from "vue";

import type { ResizablePanel } from "~/components/ui/resizable";
import { type ImageItemForDisplay, OutputImgFormat, type ProcessingConfig } from "~/composables/types";

export const displayMode = ref("horizontal" as "horizontal" | "vertical");

export type JpegFileHash = string;
export const imageDisplayList: Ref<Map<JpegFileHash, ImageItemForDisplay>> = ref(new Map());

export const db = await openDB("artefact", 20250109, {
	upgrade(db, oldVersion, newVersion) {
		const alreadyExists = db.objectStoreNames.contains("files");

		if (newVersion !== null && alreadyExists && oldVersion !== newVersion) {
			db.deleteObjectStore("files");
		}

		if (!db.objectStoreNames.contains("files")) {
			db.createObjectStore("files", {
				keyPath: "jpegFileHash",
				autoIncrement: false,
			});
		}
	},
});

export const colorScheme = ref<"light" | "dark">("light");

export const useImageCompareStore = defineStore("image-compare", () => {
	const jpegBlobUrl = ref<string | undefined>(undefined);
	const outputImgBlobUrl = ref<string | undefined>(undefined);
	const imageInputPanelRef = ref<InstanceType<typeof ResizablePanel>>();
	const compareMode = ref<"side-by-side" | "overlay">("overlay");

	return {
		jpegBlobUrl,
		outputImgBlobUrl,
		imageInputPanelRef,
		compareMode,
	};
});

export const useProcessingConfig = defineStore("processing-config", () => {
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
			// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
			const parsedConfig: ProcessingConfig = JSON.parse(config);
			outputFormat.value = parsedConfig.outputFormat;
			iterations.value = parsedConfig.iterations;
			weight.value = parsedConfig.weight;
			pWeight.value = parsedConfig.pWeight;
			separateComponents.value = parsedConfig.separateComponents;
		} else {
			localStorage.setItem("processing-config", JSON.stringify({
				outputFormat: outputFormat.value,
				iterations: iterations.value,
				weight: weight.value,
				pWeight: pWeight.value,
				separateComponents: separateComponents.value,
			}));
		}
	} catch {
		localStorage.removeItem("processing-config");
	}

	const isDefault = ref(true);
	watchEffect(() => {
		isDefault.value = (
			outputFormat.value === __DEFAULT_OUTPUT_FORMAT
			&& iterations.value === __DEFAULT_ITERATIONS
			&& weight.value === __DEFAULT_WEIGHT
			&& pWeight.value === __DEFAULT_P_WEIGHT
			&& separateComponents.value === __DEFAULT_SEPARATE_COMPONENTS
		);
	});

	watchEffect(() => {
		localStorage.setItem("processing-config", JSON.stringify({
			outputFormat: outputFormat.value,
			iterations: iterations.value,
			weight: weight.value,
			pWeight: pWeight.value,
			separateComponents: separateComponents.value,
		}));
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
		ensureInterationsValid: (): void => {
			if (iterations.value < 1) {
				iterations.value = 1;
			}
			if (iterations.value > 1000) {
				iterations.value = 1000;
			}
		},
		ensureWeightValid: (): void => {
			if (weight.value < 0) {
				weight.value = 0;
			}
			if (weight.value > 1) {
				weight.value = 1;
			}
		},
		ensurePWeightValid: (): void => {
			if (pWeight.value < 0) {
				pWeight.value = 0;
			}
			if (pWeight.value > 1) {
				pWeight.value = 1;
			}
		},
	};
});

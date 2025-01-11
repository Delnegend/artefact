import { defineStore } from "pinia";
import { ref } from "vue";

export const useImageCompareStore = defineStore("image-compare", () => {
	const jpegBlobUrl = ref<string | undefined>(undefined);
	const outputImgBlobUrl = ref<string | undefined>(undefined);
	const compareMode = ref<"side-by-side" | "overlay">("overlay");

	return {
		jpegBlobUrl,
		outputImgBlobUrl,
		compareMode,
	};
});

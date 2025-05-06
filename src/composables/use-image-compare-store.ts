import { useState } from "nuxt/app";
import type { Ref } from "vue";

interface Internal {
	jpegBlobUrl: string | undefined;
	outputImgBlobUrl: string | undefined;
	compareMode: "side-by-side" | "overlay";
}

export function useImageCompareStore(): Ref<Internal> {
	return useState<Internal>("image-compare", () => ({
		jpegBlobUrl: undefined,
		outputImgBlobUrl: undefined,
		compareMode: "overlay",
	}));
}

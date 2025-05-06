import { useState } from "nuxt/app";

export function useImageCompareStore() {
	return useState("image-compare", () => ({
		jpegBlobUrl: undefined as string | undefined,
		outputImgBlobUrl: undefined as string | undefined,
		compareMode: "overlay" as "side-by-side" | "overlay",
	}));
}
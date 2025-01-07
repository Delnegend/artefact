import { openDB } from "idb";
import { type Ref, ref } from "vue";

import type { ImageItemForDisplay } from "./types";

export const displayMode = ref("horizontal" as "horizontal" | "vertical");

export type JpegFileHash = string;
export const imageDisplayList: Ref<Map<JpegFileHash, ImageItemForDisplay>> = ref(new Map());

export const db = await openDB("artefact", 20250105, {
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

export const imageCompareImages = ref({
	jpegBlobUrl: undefined as string | undefined,
	pngBlobUrl: undefined as string | undefined,
});

export const imageCompareMode = ref("overlay" as "side-by-side" | "overlay");

import { openDB } from "idb";

import type { ImageItemForDisplay } from "./types";

export const displayMode = ref("horizontal" as "horizontal" | "vertical");

export const imageDisplayList = reactive({

	/** Key: hash */
	// eslint-disable-next-line @typescript-eslint/consistent-type-assertions
	value: {} as Record<string, ImageItemForDisplay>,
});

export const db = await openDB("artefact", 20241231, {
	upgrade(db) {
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


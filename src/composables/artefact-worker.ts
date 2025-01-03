import init, { compute } from "./artefact-wasm";
import { db } from "./states";
import type { ImageItemForDB } from "./types";

type ImageFileHash = string;

self.onmessage = async (event: MessageEvent<ImageFileHash>): Promise<void> => {
	// eslint-disable-next-line no-unused-vars
	const [_, imageInDB] = await Promise.all([init(), db.get("files", event.data) as Promise<ImageItemForDB | undefined>]);
	if (!imageInDB) {
		self.postMessage({ error: "Image not found in the database." });
		return;
	}

	// get png data
	let pngDataArray: Uint8Array;
	try {
		pngDataArray = compute(new Uint8Array(imageInDB.jpegArrayBuffer));
	} catch (e) {
		self.postMessage({ error: e });
		return;
	}

	// update in db
	void (async (): Promise<void> => {
		const tx = db.transaction("files", "readwrite");
		const store = tx.objectStore("files");
		const item = await store.get(event.data) as ImageItemForDB | undefined;
		if (item) {
			item.pngArrayBuffer = pngDataArray.buffer as ArrayBuffer;
			await store.put(item);
		}
		await tx.done;
	})();

	// create blob
	const blob = new Blob([pngDataArray], { type: "image/png" });
	const blobUrl = URL.createObjectURL(blob);

	// respond
	self.postMessage({ blobUrl });
};

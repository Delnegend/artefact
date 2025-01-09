import init, { compute, type OutputFormat } from "./artefact-wasm";
import { db } from "./states";
import { type ImageItemForDB, OutputImgFormat, type WorkerInput, type WorkerOutput } from "./types";

self.onmessage = async (event: MessageEvent<WorkerInput>): Promise<void> => {
	const { jpegFileHash, config } = event.data;

	// eslint-disable-next-line no-unused-vars
	const [_, imageInDB] = await Promise.all([init(), db.get("files", jpegFileHash) as Promise<ImageItemForDB | undefined>]);
	if (!imageInDB) {
		self.postMessage({ error: "Image not found in the database." });
		return;
	}

	const format = ((): OutputFormat => {
		switch (config.outputFormat) {
			case OutputImgFormat.PNG: return 0;
			case OutputImgFormat.WEBP: return 1;
			case OutputImgFormat.TIF: return 2;
			case OutputImgFormat.BMP: return 3;
			default: return 0;
		}
	})();

	let outputImgDataArray: Uint8Array;
	let timer = new Date().getTime();
	try {
		outputImgDataArray = compute(new Uint8Array(imageInDB.jpegArrayBuffer), format, config.weight, config.pWeight, config.iterations, config.separateComponents);
		timer = new Date().getTime() - timer;
	} catch (e) {
		const resp: WorkerOutput = { error: `${e}` };
		self.postMessage(resp);
		return;
	}

	// update in db
	void (async (): Promise<void> => {
		const tx = db.transaction("files", "readwrite");
		const store = tx.objectStore("files");
		const item = await store.get(jpegFileHash) as ImageItemForDB | undefined;
		if (item) {
			item.outputImgArrayBuffer = outputImgDataArray.buffer as ArrayBuffer;
			item.outputImgFormat = config.outputFormat;
			await store.put(item);
		}
		await tx.done;
	})();

	// create blob
	const blob = new Blob([outputImgDataArray], {
		type: `image/${config.outputFormat}`,
	});
	const blobUrl = URL.createObjectURL(blob);

	// respond
	const resp: WorkerOutput = {
		blobUrl, timeTakenInMs: timer,
		outputFormat: config.outputFormat,
	};
	self.postMessage(resp);
};

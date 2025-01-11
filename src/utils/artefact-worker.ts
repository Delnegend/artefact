import { db, type ImageItemForDB, type InputWrapperForWorker, OutputImgFormat, type OutputWrapperForWorker, type WorkerInput, WorkerMessageType, type WorkerOutput } from ".";
import init, { compute, type OutputFormat } from "./artefact-wasm";

self.onerror = (event): boolean => {
	console.error(event);

	return true;
};

self.onmessage = async (event: MessageEvent<InputWrapperForWorker<WorkerInput>>): Promise<void> => {
	switch (event.data.type) {
		case WorkerMessageType.Ping: {
			if (process.env.NODE_ENV === "development") { console.log("Worker: 🏓 Pong"); }

			const output: OutputWrapperForWorker<WorkerOutput> = { type: WorkerMessageType.Ping };
			self.postMessage(output);
			return;
		}
		case WorkerMessageType.Process: {
			if (process.env.NODE_ENV === "development") { console.log("Worker: ⏳ Task start"); }

			if (!event.data.data) { return; }
			const { config, jpegFileHash } = event.data.data;

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
				outputImgDataArray = compute(
					new Uint8Array(imageInDB.jpegArrayBuffer),
					format,
					config.weight,
					config.pWeight,
					config.iterations,
					config.separateComponents,
				);
				timer = new Date().getTime() - timer;

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
				const output: OutputWrapperForWorker<WorkerOutput> = {
					type: WorkerMessageType.Process,
					data: {
						blobUrl,
						timeTaken: `${(timer / 1_000).toFixed(2)}s`,
						outputFormat: config.outputFormat,
					},
				};
				self.postMessage(output);
				if (process.env.NODE_ENV === "development") { console.log("Worker: ✅ Task end"); }
			} catch (e) {
				const output: OutputWrapperForWorker<WorkerOutput> = {
					type: WorkerMessageType.Process,
					error: `${e}`,
				};
				self.postMessage(output);
			}
		}
	}
};

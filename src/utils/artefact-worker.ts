import init, { compute, type OutputFormat } from "./artefact-wasm/artefact_wasm";
import { getFileInDb, putFilesInDb } from "./db";
import { OutputImgFormat, type WorkerInput, type WorkerOutput } from "./types";

self.onerror = (event): boolean => {
	console.error(event);
	self.postMessage({
		type: "error",
		error: typeof event === "string" ? event : "Unknown error",
	} satisfies WorkerOutput);

	return true;
};

self.onmessage = async (event: MessageEvent<WorkerInput>): Promise<void> => {
	self.postMessage(await (async (): Promise<WorkerOutput> => {
		const { config, jpegFileHash } = event.data;

		const [_, imageInDB] = await Promise.all([
			init(),
			getFileInDb(jpegFileHash),
		]);
		if (!imageInDB) {
			return { type: "error", error: "Image not found in the database." };
		}

		const format = ((): OutputFormat => {
			switch (config.outputFormat) {
				case OutputImgFormat.PNG:
					return 0;
				case OutputImgFormat.WEBP:
					return 1;
				case OutputImgFormat.TIF:
					return 2;
				case OutputImgFormat.BMP:
					return 3;
				default:
					return 0;
			}
		})();

		let outputImgDataArray: Uint8Array;
		let timer = Date.now();
		try {
			outputImgDataArray = compute(
				new Uint8Array(imageInDB.jpegArrayBuffer),
				format,
				config.weight,
				config.pWeight,
				config.iterations,
				config.separateComponents,
			);
			timer = Date.now() - timer;

			// update in db
			await putFilesInDb([{
				...imageInDB,
				outputImgArrayBuffer: outputImgDataArray.buffer as ArrayBuffer,
				outputImgFormat: config.outputFormat,
			}]);

			// respond
			return {
				type: "process",
				timeTaken: `${(timer / 1_000).toFixed(2)}s`,
				outputFormat: config.outputFormat,
			};
		} catch (e) {
			return { type: "error", error: `${e}` };
		}
	})());
};

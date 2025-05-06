import init, { compute, type OutputFormat } from "./artefact-wasm";
import { db } from "./db";
import type { WorkerInputWrapper, WorkerOutputWrapper } from "./simple-worker-pool";
import { type ImageItemForDB, OutputImgFormat, type WorkerInput, type WorkerOutput } from "./types";

type Input = WorkerInputWrapper<WorkerInput>;
type Output = WorkerOutputWrapper<WorkerOutput>;

self.addEventListener("error", (event): boolean => {
	console.error(event);

	return true;
});

async function handleMessage(event: MessageEvent<Input>): Promise<Output> {
	if (event.data.type === "ping") {
		return { type: "pong" };
	}
	if (!event.data.data) {
		return { type: "process", error: "No data provided." };
	}
	const { config, jpegFileHash } = event.data.data;

	const [_, imageInDB] = await Promise.all([
		init(),
		db.get("files", jpegFileHash) as Promise<ImageItemForDB | undefined>,
	]);
	if (!imageInDB) {
		return { type: "process", error: "Image not found in the database." };
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
		void (async (): Promise<void> => {
			const tx = db.transaction("files", "readwrite");
			const store = tx.objectStore("files");
			const item = (await store.get(jpegFileHash)) as
				| ImageItemForDB
				| undefined;
			if (item) {
				item.outputImgArrayBuffer = outputImgDataArray.buffer as ArrayBuffer;
				item.outputImgFormat = config.outputFormat;
				await store.put(item);
			}
			await tx.done;
		})();

		// respond
		return {
			type: "process",
			data: {
				timeTaken: `${(timer / 1_000).toFixed(2)}s`,
				outputFormat: config.outputFormat,
			},
		};
	} catch (e) {
		return { type: "process", error: `${e}` };
	}
}

self.addEventListener("message", async (event: MessageEvent<Input>): Promise<void> => {
	self.postMessage(await handleMessage(event), self.location.origin);
});

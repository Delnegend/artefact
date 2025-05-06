import type { WorkerInput, WorkerOutput } from "~/utils/types";

const queue: Array<(_: Worker)=> void> = [];
const workers: Worker[] = [];
const MAX_WORKERS = 4;

function createWorker(): Worker {
	const worker = new Worker(
		new URL("./artefact-worker.ts", import.meta.url),
		{ type: "module" },
	);
	worker.onerror = (event): void => {
		console.error("Worker error:", event);
	};
	return worker;
}

for (let i = 0; i < MAX_WORKERS; i++) {
	workers.push(createWorker());
}

export function runArtefactWorker(input: WorkerInput): Promise<WorkerOutput> {
	return new Promise((resolve) => {
		function task(worker: Worker): void {
			function cleanup(): void {
				worker.onmessage = null;
				worker.onerror = null; // Reset general worker error handler if it was task-specific
			}

			function processNext(w: Worker): void {
				const nextTaskItem = queue.shift();
				if (nextTaskItem) {
					nextTaskItem(w);
				} else {
					workers.push(w);
				}
			}

			function messageHandler(event: MessageEvent<WorkerOutput>): void {
				cleanup();
				resolve(event.data);
				processNext(worker);
			}

			function errorHandler(event: ErrorEvent): void {
				console.error("Error in task execution:", event.message);
				cleanup();
				// reject(new Error(event.message || "Worker task failed"));
				resolve({ type: "error", error: event.message });
				processNext(worker);
			}

			worker.onmessage = messageHandler;
			worker.onerror = errorHandler;

			try {
				worker.postMessage(input);
			} catch (e) {
				cleanup();
				resolve({ type: "error", error: e as string });
				processNext(worker);
			}
		}

		const availableWorker = workers.shift();

		if (availableWorker) {
			task(availableWorker);
		} else {
			queue.push(task);
		}
	});
}

import { type Ref, ref } from "vue";

import { SimpleWorkerPool, type WorkerInputWrapper, type WorkerOutputWrapper } from "~/utils/simple-worker-pool";
import type { WorkerInput, WorkerOutput } from "~/utils/types";

const pool = new SimpleWorkerPool<WorkerInput, WorkerOutput>(4, () => {
	return new Worker(new URL("~/utils/simple-artefact-worker.ts", import.meta.url), {
		type: "module",
		name: "simple-artefact-worker",
	});
});

export function useSimpleArtefactWorker(input: WorkerInput): {
	output: Ref<WorkerOutput | null>;
	error: Ref<ErrorEvent | null | string>;
	processing: Ref<boolean>;
	process: ()=> void;
	terminate: ()=> void;
} {
	const output = ref<null | WorkerOutput>(null);
	const error = ref<null | ErrorEvent | string>(null);
	const processing = ref(false);

	let abortFn: (()=> void) | null = null;
	const abortSignal = new Promise<void>((resolve) => {
		abortFn = resolve;
	});

	return {
		output,
		error,
		processing,
		process(): void {
			void Promise.race([(async (): Promise<void> => {
				function onmessage(event: MessageEvent): void {
					processing.value = false;

					const wo = event.data as WorkerOutputWrapper<WorkerOutput, string>;
					if (wo.type === "pong") { return; }
					if (wo.error !== undefined) {
						error.value = wo.error;
						return;
					}
					if (!wo.data) {
						error.value = "Worker didn't return any data";
						return;
					}
					output.value = wo.data;
				}

				const worker = await pool.getWorker(abortSignal, onmessage);
				processing.value = true;

				const wi: WorkerInputWrapper<WorkerInput> = { type: "process", data: input };
				worker.postMessage(wi);
			})(), abortSignal]);
		},
		terminate(): void {
			if (abortFn !== null) { abortFn(); }
		},
	};
}

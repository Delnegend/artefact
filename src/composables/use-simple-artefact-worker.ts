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

	let abortFn: ((_: "abort")=> void) | null = null;
	const abortSignal = new Promise<"abort">((resolve) => {
		abortFn = resolve;
	});
	let completeFn: (()=> void) | null = null;
	const completeSignal = new Promise<void>((resolve) => {
		completeFn = resolve;
	});

	return {
		output,
		error,
		processing,
		async process(): Promise<void> {
			const onmessage = (event: MessageEvent): void => {
				processing.value = false;
				completeFn?.();
				const wo = event.data as WorkerOutputWrapper<WorkerOutput, string>;
				if (wo.type === "pong") { return; }
				if (wo.error !== undefined) { error.value = wo.error; return; }
				if (!wo.data) { error.value = "Worker returns no data"; return; }
				output.value = wo.data;
			};
			const worker = await pool.getWorker(onmessage);

			const wi: WorkerInputWrapper<WorkerInput> = { type: "process", data: input };
			worker.postMessage(wi);
			processing.value = true;

			if (await Promise.race([abortSignal, completeSignal]) === "abort") {
				worker.terminate();
				processing.value = false;
				await pool.releaseNewWorker(onmessage);
				return;
			}

			pool.releaseExistingWorker(worker);
		},
		terminate(): void {
			abortFn?.("abort");
		},
	};
}

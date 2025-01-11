import { type Ref, ref } from "vue";

import { type PoolTask, type WorkerInput, type WorkerOutput, WorkerPool } from "~/utils";

export const workerPool = new WorkerPool<WorkerInput, WorkerOutput>({
	workerCreator: (): Worker => new Worker(
		new URL("~/utils/artefact-worker.ts", import.meta.url),
		{ type: "module", name: "artefact-worker" },
	),
	employeeCount: 4,
});

export function useArtefactWorker(input: WorkerInput): {
	output: Ref<WorkerOutput | null>;
	error: Ref<ErrorEvent | null>;
	processing: Ref<boolean>;
	process: ()=> void;
	terminate: ()=> void;
} {
	const output = ref(null);
	const error = ref(null);
	const processing = ref(false);

	const task: PoolTask<WorkerInput, WorkerOutput> = {
		input,
		output,
		error,
		processing,
	};
	workerPool.addTask(task);

	return {
		output,
		error,
		processing,
		process: (): void => { workerPool.startQueue(); },
		terminate: (): void => { workerPool.terminateTask(task); },
	};
}

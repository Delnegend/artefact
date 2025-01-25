import { type Ref, ref } from "vue";

import type { WorkerInput, WorkerOutput } from "~/utils/types";
import { EngineerPool, type EngineerTask } from "~/utils/worker-pool";

export const workerPool = new EngineerPool<WorkerInput, WorkerOutput>({
	hireEngineer: (): Worker => new Worker(
		new URL("~/utils/artefact-worker.ts", import.meta.url),
		{ type: "module", name: "artefact-worker" },
	),
	maxActiveEngineer: 4,
	hireEngineerTimeout: 10000, // 10 seconds
	layoffDelayInMs: 300000, // 5 minutes
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

	const task: EngineerTask<WorkerInput, WorkerOutput> = {
		input,
		output,
		error,
		processing,
	};

	return {
		output,
		error,
		processing,
		process: (): void => { workerPool.addTask(task); workerPool.startQueue(); },
		terminate: (): void => { workerPool.terminateTask(task); },
	};
}

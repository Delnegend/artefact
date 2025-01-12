/* eslint-disable @typescript-eslint/no-unused-expressions, no-unused-vars */

import type { Ref } from "vue";

export enum WorkerMessageType {
	Ping = "ping", // this to make sure the "onmessage" fn is registerd in main thread
	Process = "process", // when your data actually being processed
}

/** You MUST USE this as input FOR WORKER */
export interface InputWrapperForWorker<I> {
	type: WorkerMessageType;
	data?: I;
}

/** You MUST USE this as output FOR WORKER */
export interface OutputWrapperForWorker<O, E = unknown> {
	type: WorkerMessageType;
	error?: E | ErrorEvent | string;
	data?: O;
}

export interface EngineerTask<I, O, E = unknown> {
	input: I;
	output: Ref<O | null>;
	error: Ref<ErrorEvent | string | E | null>;
	processing: Ref<boolean>;

	__DO_NOT_TOUCH_THIS?: Engineer;
}

interface Engineer {
	worker: Worker;
	isIdle: boolean;
	layoffPlanID?: number;
}

export class EngineerPool<I, O, E = unknown> {
	private readonly engineerPool: Engineer[] = [];
	private readonly taskQueue: Array<EngineerTask<I, O, E>> = [];

	private readonly hireEngineer: ()=> Worker;
	private readonly hireEngineerTimeout?: number;
	private readonly checkWorkerReadyInterval: number;
	private readonly maxActiveEngineer?: number;
	private readonly layoffDelayInMs?: number;

	public constructor(props: {
		hireEngineer: ()=> Worker;
		hireEngineerTimeout?: number;
		checkWorkerReadyInterval?: number;
		maxActiveEngineer?: number;
		layoffDelayInMs?: number;
	}) {
		this.hireEngineer = props.hireEngineer;
		this.hireEngineerTimeout = props.hireEngineerTimeout;
		this.checkWorkerReadyInterval = props.checkWorkerReadyInterval ?? 0;
		this.maxActiveEngineer = props.maxActiveEngineer;
		this.layoffDelayInMs = props.layoffDelayInMs;
	}

	private layoff(worker: Engineer): void {
		const idx = this.engineerPool.indexOf(worker);
		if (idx !== -1) { this.engineerPool.splice(idx, 1); }
		worker.worker.terminate();
		import.meta.dev && console.log("Pool: 👋 Laid off engineer");
	}

	// eslint-disable-next-line class-methods-use-this, @typescript-eslint/class-methods-use-this
	private markAsBusyAndCancelLayoffPlan(worker: Engineer): void {
		worker.isIdle = false;
		if (worker.layoffPlanID === undefined) { return; }
		window.clearTimeout(worker.layoffPlanID);
		import.meta.dev && console.log("Pool: 🏠 Cancelled layoff plan for engineer");
	}

	private markAsIdleAndPlanLayoff(worker: Engineer): void {
		worker.isIdle = true;
		if (this.layoffDelayInMs === undefined) { return; }
		worker.layoffPlanID = window.setTimeout(
			() => { this.layoff(worker); },
			this.layoffDelayInMs,
		);

		import.meta.dev && console.log("Pool: 🏠 Engineer will go home soon");
	}

	/** Returns a ready-to-work engineer, but remember to
	 * lay off next morning if you don't need it anymore. */
	private async sendAnEngineer(): Promise<Engineer | undefined> {
		let engineer = this.engineerPool.find(engineer => engineer.isIdle);

		// found one available
		if (engineer !== undefined) {
			this.markAsBusyAndCancelLayoffPlan(engineer);
			import.meta.dev && console.log("Pool: 👷‍♂️ Found an existing engineer");
			return engineer;
		}

		// can't hire more
		console.log(
			"maxActiveEngineer", this.maxActiveEngineer, "engineerPool.length", this.engineerPool.length, "engineerPool", this.engineerPool,
		);
		if (this.maxActiveEngineer !== undefined && (this.engineerPool.length >= this.maxActiveEngineer)) {
			import.meta.dev && console.log("Pool: 🚫 Can't hire more engineers");
			return;
		}

		// or CAN we?
		engineer = { worker: this.hireEngineer(), isIdle: false };
		this.engineerPool.push(engineer);
		import.meta.dev && console.log("Pool: 🤝 Hired a new engineer");

		const pollingIntervalID = window.setInterval(() => {
			engineer.worker.postMessage({ type: WorkerMessageType.Ping });
		}, this.checkWorkerReadyInterval);

		// wait for the worker to be ready
		await new Promise((resolve, reject) => {
			let hireEngineerTimeoutID: number | undefined;
			if (this.layoffDelayInMs !== undefined) {
				hireEngineerTimeoutID = window.setTimeout(() => {
					reject(new Error("Engineer creation timeout"));
				}, this.hireEngineerTimeout);
			}

			engineer.worker.onmessage = (event): void => {
				const output = event.data as OutputWrapperForWorker<O, E>;
				if (output.type === WorkerMessageType.Ping) {
					if (hireEngineerTimeoutID !== undefined) {
						window.clearTimeout(hireEngineerTimeoutID);
					}
					window.clearInterval(pollingIntervalID);
					import.meta.dev && console.log("Pool: 🎉 Engineer is ready");
					resolve({});
				}
			};
		});

		return engineer;
	}

	private async processNext(): Promise<void> {
		if (this.taskQueue.length === 0) { return; }

		const engineer = await this.sendAnEngineer();
		if (engineer === undefined) { return; }

		const task = this.taskQueue.shift();
		// someone else took the task
		if (task === undefined) {
			this.markAsIdleAndPlanLayoff(engineer);
			return;
		}

		task.processing.value = true;
		task.__DO_NOT_TOUCH_THIS = engineer;

		const postProcess = (): void => {
			task.processing.value = false;
			task.__DO_NOT_TOUCH_THIS = undefined;
			this.markAsIdleAndPlanLayoff(engineer);
			void this.processNext();
		};

		engineer.worker.onmessage = (event): void => {
			const workerOutput = event.data as OutputWrapperForWorker<O, E>;
			switch (workerOutput.type) {
				case WorkerMessageType.Process:
				{
					if (workerOutput.error !== undefined) {
						task.error.value = workerOutput.error;
						break;
					}
					if (workerOutput.data === undefined) {
						task.error.value = "Worker didn't return any data";
						break;
					}
					task.output.value = workerOutput.data;
					break;
				}
				case WorkerMessageType.Ping:
				{
					this.taskQueue.unshift(task);
					break;
				}
			}
			postProcess();
		};

		engineer.worker.onerror = (error): void => {
			task.error.value = error;
			postProcess();
		};

		engineer.worker.postMessage({
			type: WorkerMessageType.Process,
			data: task.input,
		});
	}

	public addTask(task: EngineerTask<I, O, E>): void {
		task.processing.value = false;
		this.taskQueue.push(task);
	}

	public startQueue(): void {
		void this.processNext();
	}

	public terminateTask(task: EngineerTask<I, O, E>): void {
		if (task.__DO_NOT_TOUCH_THIS) {
			this.layoff(task.__DO_NOT_TOUCH_THIS);
		}

		const taskIdx = this.taskQueue.indexOf(task);
		if (taskIdx !== -1) { this.taskQueue.splice(taskIdx, 1); }
	}
}

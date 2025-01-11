import type { Ref } from "vue";

export enum WorkerMessageType {
	// eslint-disable-next-line no-unused-vars
	Ping = "ping", // this to make sure the "onmessage" fn is registerd in main thread
	// eslint-disable-next-line no-unused-vars
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

export interface PoolTask<I, O, E = unknown> {
	input: I;
	output: Ref<O | null>;
	error: Ref<ErrorEvent | string | E | null>;
	processing: Ref<boolean>;

	__DO_NOT_TOUCH_THIS?: Engineer;
}

interface Engineer {
	worker: Worker;
	available: boolean;
	countdownGoHomeID?: number;
}

export class WorkerPool<I, O, E = unknown> {
	private readonly engineerPool: Engineer[] = [];
	private readonly taskQueue: Array<PoolTask<I, O, E>> = [];

	private readonly workerCreator: ()=> Worker;
	private readonly employeeCount: number;
	private readonly countdownGoHomeInMs: number;
	private readonly hiringTimeoutInMs: number;
	private readonly isWorkerReadyYetInMs: number;

	public constructor(props: {
		workerCreator: ()=> Worker;
		employeeCount: number;
		countdownGoHomeInMs?: number;
		hiringTimeoutInMs?: number;
		isWorkerReadyYetInMs?: number;
	}) {
		this.workerCreator = props.workerCreator;
		this.employeeCount = props.employeeCount;
		this.countdownGoHomeInMs = props.countdownGoHomeInMs ?? 10000;
		this.hiringTimeoutInMs = props.hiringTimeoutInMs ?? 5000;
		this.isWorkerReadyYetInMs = props.isWorkerReadyYetInMs ?? 50;
	}

	private layoff(engineer: Engineer): void {
		const idx = this.engineerPool.indexOf(engineer);
		if (idx !== -1) { this.engineerPool.splice(idx, 1); }

		engineer.worker.terminate();
		if (process.env.NODE_ENV === "development") {
			console.log("Pool: 👋 Laid off engineer");
		}
	}

	// eslint-disable-next-line class-methods-use-this, @typescript-eslint/class-methods-use-this
	private cancelLayoffPlan(engineer: Engineer): void {
		window.clearTimeout(engineer.countdownGoHomeID);
		engineer.available = false;
		if (process.env.NODE_ENV === "development") {
			console.log("Pool: 🏠 Cancelled layoff plan for engineer");
		}
	}

	private layoffNextMorning(engineer: Engineer): void {
		engineer.available = true;
		engineer.countdownGoHomeID = window.setTimeout(
			() => { this.layoff(engineer); },
			this.countdownGoHomeInMs,
		);
		if (process.env.NODE_ENV === "development") {
			console.log("Pool: 🏠 Engineer will go home soon");
		}
	}

	private async sendAnEngineer(): Promise<Engineer | undefined> {
		let engineer = this.engineerPool.find(engineer => engineer.available);
		if (engineer !== undefined) {
			if (process.env.NODE_ENV === "development") {
				console.log("Pool: 👷‍♂️ Found an existing engineer");
			}
			return engineer;
		}

		// can't hire more
		if (this.engineerPool.length >= this.employeeCount) {
			if (process.env.NODE_ENV === "development") {
				console.log("Pool: 🚫 Can't hire more engineers");
			}
			return;
		}

		// or CAN we?
		engineer = { worker: this.workerCreator(), available: false };
		this.engineerPool.push(engineer);

		if (process.env.NODE_ENV === "development") {
			console.log("Pool: 🤝 Hired a new engineer");
		}

		// just in case someone else takes the task before this one
		this.layoffNextMorning(engineer);

		// turns out postMessage and onmessage is async, setting onmessage
		// before using postMessage doesn't guarantee that the worker is ready
		const msgSpamIntervalID = window.setInterval(() => {
			engineer.worker.postMessage({ type: WorkerMessageType.Ping });
		}, this.isWorkerReadyYetInMs);

		return new Promise((resolve, reject) => {
			const creationTimeoutID = window.setTimeout(() => {
				reject(new Error("Engineer creation timeout"));
			}, this.hiringTimeoutInMs);

			engineer.worker.onmessage = (event): void => {
				const output = event.data as OutputWrapperForWorker<O, E>;

				if (output.type === WorkerMessageType.Ping) {
					window.clearTimeout(creationTimeoutID);
					window.clearInterval(msgSpamIntervalID);

					if (process.env.NODE_ENV === "development") {
						console.log("Pool: 🎉 Engineer is ready");
					}

					resolve(engineer);
				}
			};
		});
	}

	private async processNext(): Promise<void> {
		if (this.taskQueue.length === 0) { return; }

		// hello we got work to do

		const engineer = await this.sendAnEngineer();
		if (engineer === undefined) { return; }

		const task = this.taskQueue.shift();
		if (task === undefined) { return; }

		this.cancelLayoffPlan(engineer);
		task.__DO_NOT_TOUCH_THIS = engineer;
		task.processing.value = true;

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

			task.processing.value = false;
			task.__DO_NOT_TOUCH_THIS = undefined;
			this.layoffNextMorning(engineer);
			void this.processNext();
		};

		engineer.worker.onerror = (error): void => {
			task.error.value = error;

			task.processing.value = false;
			task.__DO_NOT_TOUCH_THIS = undefined;
			this.layoffNextMorning(engineer);
			void this.processNext();
		};
		engineer.worker.postMessage({
			type: WorkerMessageType.Process,
			data: task.input,
		});
	}

	public addTask(task: PoolTask<I, O, E>): void {
		this.taskQueue.push(task);
	}

	public startQueue(): void {
		void this.processNext();
	}

	public terminateTask(task: PoolTask<I, O, E>): void {
		if (task.__DO_NOT_TOUCH_THIS) { this.layoff(task.__DO_NOT_TOUCH_THIS); }

		const taskIdx = this.taskQueue.indexOf(task);
		if (taskIdx !== -1) { this.taskQueue.splice(taskIdx, 1); }
	}
}

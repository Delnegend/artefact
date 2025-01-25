export interface WorkerInputWrapper<T> {
	type: "ping" | "process";
	data?: T;
}

export interface WorkerOutputWrapper<T, E = unknown> {
	type: "pong" | "process";
	data?: T;
	error?: E | ErrorEvent | string;
}

export class SimpleWorkerPool<I, O, E = unknown> {
	private readonly pool: Worker[] = [];
	private semaphore: number;
	private readonly queue: Array<(_: Worker)=> void> = [];
	private readonly newWorkerFn: ()=> Worker;

	public constructor(maxWorker: number, newWorkerFn: ()=> Worker) {
		this.semaphore = maxWorker;
		this.newWorkerFn = newWorkerFn;
	}

	// eslint-disable-next-line @typescript-eslint/class-methods-use-this, class-methods-use-this
	private async ensureWorkerReady(worker: Worker, onmessage: (_: MessageEvent)=> void): Promise<Worker> {
		const polling = window.setInterval(() => {
			const input: WorkerInputWrapper<I> = { type: "ping" };
			worker.postMessage(input);
		}, 0);

		return new Promise((resolve) => {
			worker.onmessage = (event): void => {
				onmessage(event);
				const out = event.data as WorkerOutputWrapper<O, E>;
				if (out.type === "pong") {
					window.clearInterval(polling);
					resolve(worker);
				}
			};
		});
	}

	public async getWorker(onmessage: (_: MessageEvent)=> void): Promise<Worker> {
		const worker = this.pool.shift();
		if (worker) {
			return this.ensureWorkerReady(worker, onmessage);
		}

		if (this.semaphore > 0) {
			this.semaphore--;
			return this.ensureWorkerReady(this.newWorkerFn(), onmessage);
		}

		return new Promise((resolve) => {
			this.queue.unshift((worker) => { resolve(worker); });
		});
	}

	public releaseExistingWorker(worker: Worker): void {
		const next = this.queue.shift();
		if (next) { next(worker); return; }
		this.pool.push(worker);
		this.semaphore++;
	}

	public async releaseNewWorker(onmessage: (_: MessageEvent)=> void): Promise<void> {
		const worker = await this.ensureWorkerReady(this.newWorkerFn(), onmessage);
		const next = this.queue.shift();
		if (next) { next(worker); return; }
		this.pool.push(worker);
		this.semaphore++;
	}
}

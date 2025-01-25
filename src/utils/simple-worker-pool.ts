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
	private remaining: number;
	private readonly queue: Array<(_: Worker)=> void> = [];
	private readonly newWorkerFn: ()=> Worker;

	public constructor(maxWorker: number, newWorkerFn: ()=> Worker) {
		this.remaining = maxWorker;
		this.newWorkerFn = newWorkerFn;
	}

	private async createWorker(onmessage: (_: MessageEvent)=> void): Promise<Worker> {
		const worker = this.newWorkerFn();

		const polling = window.setInterval(() => {
			const input: WorkerInputWrapper<I> = { type: "ping" };
			worker.postMessage(input);
		}, 0);

		return new Promise((resolve) => {
			worker.onmessage = async (event): Promise<void> => {
				onmessage(event);
				const out = event.data as WorkerOutputWrapper<O, E>;
				if (out.type === "pong") {
					window.clearInterval(polling);
					resolve(worker);
				} else {
					await this.releaseWorker(onmessage, worker);
				}
			};
		});
	}

	public async getWorker(abort: Promise<void>, onmessage: (_: MessageEvent)=> void): Promise<Worker> {
		const worker = this.pool.shift();
		if (worker) { return worker; }

		if (this.remaining > 0) {
			this.remaining--;
			const worker = await Promise.race([this.createWorker(onmessage), abort]);
			if (!worker) {
				this.remaining++;
				await this.releaseWorker(onmessage);
				throw new Error("Worker creation aborted");
			}
			return worker;
		}

		return new Promise((resolve) => {
			this.queue.unshift((worker) => { resolve(worker); });
		});
	}

	private async releaseWorker(onmessage: (_: MessageEvent)=> void, worker?: Worker): Promise<void> {
		let worker_ = worker;
		if (!worker_) { worker_ = await this.createWorker(onmessage); }
		const next = this.queue.shift();
		if (next) { next(worker_); return; }
		this.pool.push(worker_);
		this.remaining++;
	}
}

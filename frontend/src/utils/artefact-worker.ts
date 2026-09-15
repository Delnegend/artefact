import init, { compute, type OutputFormat } from './artefact-wasm/artefact_wasm'
import { getFileInDb, putFilesInDb } from './db'
import { OutputImgFormat, type WorkerInput, type WorkerOutput } from './types'

/// WebGPU can expose `navigator.gpu` yet resolve `requestAdapter()` to `null`
/// (e.g. WebGPU disabled or blocklisted). wgpu 30 dereferences that `null`, and
/// also expects `GPUAdapter.info`; probe both and fall back to the CPU pipeline
/// otherwise.
async function canUseGpu(): Promise<boolean> {
	const gpu = (
		navigator as Navigator & {
			gpu?: { requestAdapter(): Promise<unknown> }
		}
	).gpu
	if (!gpu) {
		return false
	}
	try {
		const adapter = (await gpu.requestAdapter()) as {
			info?: unknown
		} | null
		return adapter != null && adapter.info != null
	} catch {
		return false
	}
}

self.onerror = (event): boolean => {
	console.error(event)
	self.postMessage({
		type: 'error',
		error: typeof event === 'string' ? event : 'Unknown error'
	} satisfies WorkerOutput)

	return true
}

self.onmessage = async (event: MessageEvent<WorkerInput>): Promise<void> => {
	self.postMessage(
		await (async (): Promise<WorkerOutput> => {
			const { config, jpegFileHash } = event.data

			const [_, imageInDB] = await Promise.all([
				init(),
				getFileInDb(jpegFileHash)
			])
			if (!imageInDB) {
				return {
					type: 'error',
					error: 'Image not found in the database.'
				}
			}

			const format = ((): OutputFormat => {
				switch (config.outputFormat) {
					case OutputImgFormat.PNG:
						return 0
					case OutputImgFormat.WEBP:
						return 1
					case OutputImgFormat.TIF:
						return 2
					case OutputImgFormat.BMP:
						return 3
					default:
						return 0
				}
			})()

			let outputImgDataArray: Uint8Array
			let timer = Date.now()
			try {
				const useGpu = await canUseGpu()
				console.info(
					useGpu
						? 'artefact: solving on GPU'
						: 'artefact: WebGPU unavailable, solving on CPU'
				)
				outputImgDataArray = await compute(
					new Uint8Array(imageInDB.jpegArrayBuffer),
					format,
					config.weight,
					config.pWeight,
					config.iterations,
					config.separateComponents,
					useGpu
				)
				timer = Date.now() - timer

				// update in db
				await putFilesInDb([
					{
						...imageInDB,
						outputImgArrayBuffer:
							outputImgDataArray.buffer as ArrayBuffer,
						outputImgFormat: config.outputFormat
					}
				])

				// respond
				return {
					type: 'process',
					timeTaken: `${(timer / 1_000).toFixed(2)}s`,
					outputFormat: config.outputFormat
				}
			} catch (e) {
				return { type: 'error', error: `${e}` }
			}
		})()
	)
}

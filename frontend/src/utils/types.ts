export enum OutputImgFormat {
	PNG = 'png',
	JXL = 'jxl',
	WEBP = 'webp',
	TIF = 'tif',
	BMP = 'bmp'
}

export interface ImageItemForDB {
	jpegFileHash: string
	jpegFileName: string
	dateAdded: Date
	jpegFileSize: number
	jpegArrayBuffer: ArrayBuffer
	outputImgArrayBuffer?: ArrayBuffer
	previewImgArrayBuffer?: ArrayBuffer
	outputImgFormat?: OutputImgFormat
	width: number
	height: number
}

export interface ImageItemForDisplay {
	name: string
	dateAdded: Date
	size: number
	jpegBlobUrl: string
	outputImgBlobUrl?: string
	previewImgBlobUrl?: string
	outputImgFormat?: OutputImgFormat
	width: number
	height: number
}

export interface WorkerInput {
	jpegFileHash: string
	config: ProcessingConfig
}

export type WorkerOutput =
	| {
			type: 'process'
			timeTaken?: string
			outputFormat?: OutputImgFormat
	  }
	| {
			type: 'error'
			error: string
	  }

export interface ProcessingConfig {
	outputFormat: OutputImgFormat
	png16Bit: boolean
	iterations: number
	weight: number
	pWeight: number
	separateComponents: boolean
}

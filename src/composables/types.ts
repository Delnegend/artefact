export enum OutputImgFormat {
	// eslint-disable-next-line no-unused-vars
	PNG = "png",
	// eslint-disable-next-line no-unused-vars
	WEBP = "webp",
	// eslint-disable-next-line no-unused-vars
	TIF = "tif",
	// eslint-disable-next-line no-unused-vars
	BMP = "bmp",
}

export interface ImageItemForDB {
	jpegFileHash: string;
	jpegFileName: string;
	dateAdded: Date;
	jpegFileSize: number;
	jpegArrayBuffer: ArrayBuffer;
	outputImgArrayBuffer?: ArrayBuffer;
	outputImgFormat?: OutputImgFormat;
	width: number;
	height: number;
}

export interface ImageItemForDisplay {
	name: string;
	dateAdded: Date;
	size: number;
	jpegBlobUrl: string;
	outputImgBlobUrl?: string;
	outputImgFormat?: OutputImgFormat;
	width: number;
	height: number;
}

export interface WorkerInput {
	jpegFileHash: string;
	config: ProcessingConfig;
}

export interface WorkerOutput {
	blobUrl?: string;
	error?: string;
	timeTakenInMs?: number;
	outputFormat?: OutputImgFormat;
}

export interface ProcessingConfig {
	outputFormat: OutputImgFormat;
	iterations: number;
	weight: number;
	pWeight: number;
	separateComponents: boolean;
}

/* eslint-disable no-unused-vars */
export enum OutputImgFormat {
	PNG = "png",
	WEBP = "webp",
	TIF = "tif",
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
	timeTaken?: string;
	outputFormat?: OutputImgFormat;
}

export interface ProcessingConfig {
	outputFormat: OutputImgFormat;
	iterations: number;
	weight: number;
	pWeight: number;
	separateComponents: boolean;
}

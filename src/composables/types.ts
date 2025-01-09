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
	pngArrayBuffer?: ArrayBuffer;
	width: number;
	height: number;
}

export interface ImageItemForDisplay {
	name: string;
	dateAdded: Date;
	size: number;
	jpegBlobUrl: string;
	pngBlobUrl?: string;
	width: number;
	height: number;
}

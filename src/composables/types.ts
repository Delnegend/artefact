export interface ImageItemForDB {
	jpegFileHash: string;
	jpegFileName: string;
	dateAdded: Date;
	jpegFileSize: number;
	jpegArrayBuffer: ArrayBuffer;
	pngArrayBuffer?: ArrayBuffer;
}

export interface ImageItemForDisplay {
	name: string;
	dateAdded: Date;
	size: number;
	jpegBlobUrl: string;
	pngBlobUrl?: string;
}

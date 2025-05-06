
import { useState } from "nuxt/app";
import { deleteFileInDb, getAllFilesInDb, putFilesInDb } from "~/utils/db";
import type { ImageItemForDisplay } from "~/utils/types";
import { hashArrayBuffer } from "../utils/hash-array-buffer";

export function useImageListStore() {
	return useState<{ [key: string]: ImageItemForDisplay }>("image-list", () => ({}));
}

export const imageListStoreOps = {
	async init(): Promise<void> {
		const imageListStore = useImageListStore();
		imageListStore.value = (await getAllFilesInDb()).reduce((acc, image) => {
			acc[image.jpegFileHash] = {
				name: image.jpegFileName,
				dateAdded: image.dateAdded,
				size: image.jpegFileSize,
				jpegBlobUrl: URL.createObjectURL(
					new Blob([image.jpegArrayBuffer], { type: "image/jpeg" }),
				),
				outputImgBlobUrl: image.outputImgArrayBuffer
					? URL.createObjectURL(
						new Blob([image.outputImgArrayBuffer], {
							type: `image/${image.outputImgFormat}`,
						}),
					)
					: undefined,
				outputImgFormat: image.outputImgFormat,
				width: image.width,
				height: image.height,
			};
			return acc;
		}, {} as { [key: string]: ImageItemForDisplay });
	},

	async addFileList(fileList: FileList | null): Promise<void> {
		if (!fileList) { return; }

		const imageListStore = useImageListStore();

		const fileOps = await Promise.all(
			Array.from(fileList).map(async (file) => {
				const jpegArrayBuffer = await file.arrayBuffer();
				const hash = await hashArrayBuffer(jpegArrayBuffer);

				const img = new Image();
				img.src = URL.createObjectURL(
					new Blob([jpegArrayBuffer], { type: "image/jpeg" }),
				);
				await new Promise((resolve) => {
					img.onload = resolve;
				});

				return {
					file,
					jpegArrayBuffer,
					hash,
					width: img.width,
					height: img.height,
				};
			}),
		);

		imageListStore.value = Object.fromEntries(
			fileOps.map(({ file, jpegArrayBuffer, hash, width, height }) => {
				return [
					hash,
					{
						name: file.name,
						dateAdded: new Date(),
						size: jpegArrayBuffer.byteLength,
						jpegBlobUrl: URL.createObjectURL(
							new Blob([jpegArrayBuffer], { type: "image/jpeg" }),
						),
						width,
						height,
					},
				];
			}),
		);

		putFilesInDb(fileOps.map(({ file, jpegArrayBuffer, hash, width, height }) => {
			return {
				jpegFileHash: hash,
				jpegFileName: file.name,
				dateAdded: new Date(),
				jpegFileSize: jpegArrayBuffer.byteLength,
				jpegArrayBuffer,
				width,
				height,
			};
		}));
	},

	async remove(jpegFileHash: string): Promise<void> {
		const imageListStore = useImageListStore();
		delete imageListStore.value[jpegFileHash];
		await deleteFileInDb(jpegFileHash);
	}
} as const;
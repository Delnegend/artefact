import { defineStore } from "pinia";
import { ref } from "vue";

import { db } from "~/utils/db";
import { hashArrayBuffer } from "~/utils/hash-array-buffer";
import type { ImageItemForDB, ImageItemForDisplay, OutputImgFormat } from "~/utils/types";

export const useImageDisplayListStore = defineStore("image-display-list", () => {
	type JpegFileHash = string;
	const list = ref<Map<JpegFileHash, ImageItemForDisplay>>(new Map());

	return {
		list,
		loadFromDB: async (): Promise<void> => {
			const tx = db.transaction("files", "readonly");
			const store = tx.objectStore("files");
			const files = await store.getAll() as ImageItemForDB[];

			for (const file of files) {
				const jpegBlobUrl = URL.createObjectURL(new Blob([file.jpegArrayBuffer], { type: "image/jpeg" }));
				const outputImgBlobUrl = file.outputImgArrayBuffer
					? URL.createObjectURL(new Blob([file.outputImgArrayBuffer], { type: `image/${file.outputImgFormat}` }))
					: undefined;

				list.value.set(file.jpegFileHash, {
					name: file.jpegFileName,
					dateAdded: file.dateAdded,
					size: file.jpegFileSize,
					jpegBlobUrl,
					outputImgBlobUrl,
					outputImgFormat: file.outputImgFormat,
					width: file.width,
					height: file.height,
				});
			}
		},
		addFileList: async (fileList: FileList | null): Promise<void> => {
			if (!fileList) { return; }

			const fileOps = await Promise.all(
				Array.from(fileList).map(async (file) => {
					const jpegArrayBuffer = await file.arrayBuffer();
					const hash = await hashArrayBuffer(jpegArrayBuffer);

					const img = new Image();
					img.src = URL.createObjectURL(new Blob([jpegArrayBuffer], { type: "image/jpeg" }));
					await new Promise((resolve) => { img.onload = resolve; });

					return { file, jpegArrayBuffer, hash, width: img.width, height: img.height };
				}),
			);

			const tx = db.transaction("files", "readwrite");
			const store = tx.objectStore("files");

			await Promise.all(
				fileOps.map(async ({ file, jpegArrayBuffer, hash, width, height }) => {
					const now = new Date();

					const itemToInsert: ImageItemForDB = {
						jpegFileHash: hash,
						jpegFileName: file.name,
						dateAdded: now,
						jpegFileSize: jpegArrayBuffer.byteLength,
						jpegArrayBuffer,
						width,
						height,
					};
					await store.put(itemToInsert);

					list.value.set(hash, {
						name: file.name,
						dateAdded: now,
						size: jpegArrayBuffer.byteLength,
						jpegBlobUrl: URL.createObjectURL(new Blob([jpegArrayBuffer], { type: "image/jpeg" })),
						width,
						height,
					});
				}),
			);

			await tx.done;
		},
		remove: async (jpegFileHash: string): Promise<void> => {
			list.value.delete(jpegFileHash);
			await db.delete("files", jpegFileHash);
		},
		setOutputImgBlobUrl: (jpegFileHash: JpegFileHash, outputImgBlobUrl: string | undefined): void => {
			const imageItem = list.value.get(jpegFileHash);
			if (!imageItem) { return; }

			imageItem.outputImgBlobUrl = outputImgBlobUrl;
		},
		setOutputImgFormat: (jpegFileHash: JpegFileHash, outputImgFormat?: OutputImgFormat): void => {
			const imageItem = list.value.get(jpegFileHash);
			if (!imageItem) { return; }

			imageItem.outputImgFormat = outputImgFormat;
		},
		getOutputImgBlobUrl: async (jpegFileHash: JpegFileHash): Promise<string | undefined> => {
			const imageItem = list.value.get(jpegFileHash);
			if (imageItem?.outputImgBlobUrl) { return imageItem.outputImgBlobUrl; }

			const tx = db.transaction("files", "readonly");
			const store = tx.objectStore("files");

			const imageItemForDB = await store.get(jpegFileHash) as ImageItemForDB | undefined;
			if (imageItemForDB?.outputImgArrayBuffer === undefined) {
				return;
			}

			return URL.createObjectURL(new Blob([imageItemForDB.outputImgArrayBuffer], { type: `image/${imageItemForDB.outputImgFormat}` }));
		},
	};
});

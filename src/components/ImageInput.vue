<script setup lang="ts">
import { toast } from "vue-sonner";

import { hashArrayBuffer } from "~/composables/hash";
import { db, imageDisplayList } from "~/composables/states";
import type { ImageItemForDB } from "~/composables/types";

const fileDialog = useFileDialog({ accept: "image/jpeg" });
fileDialog.onChange(async (files) => {
	if (!files) { return; }

	try {
		const fileOps = await Promise.all(
			Array.from(files).map(async (file) => {
				const jpegArrayBuffer = await file.arrayBuffer();
				const hash = await hashArrayBuffer(jpegArrayBuffer);
				return { file, jpegArrayBuffer, hash };
			}),
		);

		const tx = db.transaction("files", "readwrite");
		const store = tx.objectStore("files");

		await Promise.all(
			fileOps.map(async ({ file, jpegArrayBuffer, hash }) => {
				const now = new Date();

				const itemToInsert: ImageItemForDB = {
					jpegFileHash: hash,
					jpegFileName: file.name,
					dateAdded: now,
					jpegFileSize: jpegArrayBuffer.byteLength,
					jpegArrayBuffer,
				};
				await store.put(itemToInsert);

				imageDisplayList.value[hash] = {
					name: file.name,
					dateAdded: now,
					size: jpegArrayBuffer.byteLength,
					jpegBlobUrl: URL.createObjectURL(new Blob([jpegArrayBuffer], { type: "image/jpeg" })),
				};
			}),
		);

		await tx.done;
	} catch (error) {
		toast.error("Failed to process files", {
			description: `${error}`,
		});
	}
});
</script>

<template>
	<Button
		class="h-28 border w-[calc(100%-2rem)] m-4 text-balance text-xl border-neutral-300 border-dashed text-center"
		variant="secondary"
		@click="fileDialog.open()"
	>
		Select or drop JPEG(s) here
	</Button>
</template>

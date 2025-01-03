<script setup lang="ts">
import { toast } from "vue-sonner";

import { db, imageDisplayList } from "~/composables/states";
import type { ImageItemForDB } from "~/composables/types";

import ImageItem from "./ImageItem.vue";

// load from DB to filesQueue to render on screen
try {
	const tx = db.transaction("files", "readonly");
	const store = tx.objectStore("files");
	const files = await store.getAll() as ImageItemForDB[];

	for (const file of files) {
		imageDisplayList.value.set(file.jpegFileHash, {
			name: file.jpegFileName,
			dateAdded: file.dateAdded,
			size: file.jpegFileSize,
			jpegBlobUrl: URL.createObjectURL(new Blob([file.jpegArrayBuffer], { type: "image/jpeg" })),
			pngBlobUrl: file.pngArrayBuffer ? URL.createObjectURL(new Blob([file.pngArrayBuffer], { type: "image/png" })) : undefined,
			width: file.width,
			height: file.height,
		});
	}
} catch (error) {
	toast.error("Failed to load files", {
		description: `${error}`,
	});
}
</script>

<template>
	<div class="flex flex-col gap-4 overflow-y-auto">
		<ImageItem
			v-for="[jpegFileHash, info] in imageDisplayList"
			:key="jpegFileHash"
			:jpeg-file-hash="jpegFileHash"
			:info="info" />
	</div>
</template>

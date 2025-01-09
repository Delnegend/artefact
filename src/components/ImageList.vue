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
			outputImgBlobUrl: file.outputImgArrayBuffer
				? URL.createObjectURL(new Blob([file.outputImgArrayBuffer], { type: `image/${file.outputImgFormat}` }))
				: undefined,
			outputImgFormat: file.outputImgFormat,
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
	<TransitionGroup name="list" tag="div" class="relative">
		<div ref="dummyElementRef" key="dummy" class="w-full" />
		<ImageItem
			v-for="[jpegFileHash, info] in imageDisplayList"
			:key="jpegFileHash"
			:jpeg-file-hash="jpegFileHash"
			:info="info"
			class="mb-4" />
	</TransitionGroup>
</template>

<style scoped>
.list-move,
.list-enter-active,
.list-leave-active {
	transition: all 0.5s ease;
}

.list-enter-from,
.list-leave-to {
	opacity: 0;
	transform: translateX(30px);
}

.list-leave-active {
	position: absolute;
	overflow: hidden;
	width: 100%;
}
</style>

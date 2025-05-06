<script setup lang="ts">
import { toast } from "vue-sonner";

import { imageListStoreOps, useImageListStore } from "~/composables/use-image-list-store";

import ImageItem from "./ImageItem.vue";

const imgList = useImageListStore();

try {
	await imageListStoreOps.init();
} catch (error) {
	toast.error("Failed to load files from DB", {
		description: `${error}`,
	});
}
</script>

<template>
	<TransitionGroup
		name="list"
		tag="div"
		class="relative">
		<div
			ref="dummyElementRef"
			key="dummy"
			class="w-full" />
		<ImageItem
			v-for="[jpegFileHash, info] in Object.entries(imgList)"
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

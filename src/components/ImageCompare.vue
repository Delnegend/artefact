<script setup lang="ts">
import { Search } from "lucide-vue-next";

import { imageCompare } from "~/composables/states";

import Button from "./ui/button/Button.vue";

const imageRef = ref<HTMLImageElement | null>(null);

// ===== for scaling the image =====

const scale = ref(1);
const MIN_SCALE = 1; // DO NOT PUT 0 OR SMALLER
const MAX_SCALE = 100;
function handleWheel(e: WheelEvent): void {
	if (!imageRef.value) { return; }

	const delta = e.deltaY > 0 ? 0.9 : 1.1;
	scale.value = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale.value * delta));
}

// ===== for moving the image =====

const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const position = ref({ x: 0, y: 0 });

function startDrag(e: MouseEvent | TouchEvent): void {
	isDragging.value = true;
	const pos = "touches" in e ? e.touches[0] : e;
	dragStart.value = {
		x: pos.clientX - position.value.x * scale.value,
		y: pos.clientY - position.value.y * scale.value,
	};
}

function doDrag(e: MouseEvent | TouchEvent): void {
	if (!isDragging.value) { return; }
	const pos = "touches" in e ? e.touches[0] : e;
	position.value = {
		x: (pos.clientX - dragStart.value.x) / scale.value,
		y: (pos.clientY - dragStart.value.y) / scale.value,
	};
}

function stopDrag(): void {
	isDragging.value = false;
}

// reset scale and position when image changes

watch(imageCompare, () => {
	scale.value = 1;
	position.value = { x: 0, y: 0 };
});
</script>

<template>
	<div
		v-if="imageCompare.jpegBlobUrl && imageCompare.pngBlobUrl"
		class="grid grid-cols-2 size-full relative"
	>
		<div
			ref="imageContainerRef"
			class="cursor-grab active:cursor-grabbing relative overflow-hidden"
			@mousedown.prevent="startDrag"
			@mousemove.prevent="doDrag"
			@mouseup.prevent="stopDrag"

			@mouseleave.prevent="stopDrag"
			@touchstart="startDrag"
			@touchmove="doDrag"
			@touchend="stopDrag"

			@touchcancel="stopDrag"
		>
			<img
				ref="imageRef"
				:src="imageCompare.jpegBlobUrl"
				class="size-full object-contain select-none origin-center"
				:style="{
					transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
					transition: isDragging ? 'none' : 'transform 0.1s ease-out',
					imageRendering: 'pixelated',
				}"
				@wheel.prevent="handleWheel"
			>
		</div>
		<div
			class="cursor-grab active:cursor-grabbing relative overflow-hidden"
			@mousedown.prevent="startDrag"
			@mousemove.prevent="doDrag"
			@mouseup.prevent="stopDrag"
			@mouseleave.prevent="stopDrag"

			@touchstart="startDrag"
			@touchmove="doDrag"
			@touchend="stopDrag"
			@touchcancel="stopDrag"
		>
			<img
				ref="imageRef"
				:src="imageCompare.pngBlobUrl"
				class="size-full object-contain select-none origin-center"
				:style="{
					transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
					transition: isDragging ? 'none' : 'transform 0.1s ease-out',
					imageRendering: 'pixelated',
				}"
				@wheel.prevent="handleWheel"
			>
		</div>

		<div class="absolute bottom-4 left-5 gap-4 flex bg-transparent">
			<Button
				size="lg"
				class="p-4 shadow-sm hover:shadow-md transition-all"
				:disabled="scale === 1"
				@click="scale = 1"
			>
				<Search />
				<span class="w-7">x{{ Math.round(scale * 10) / 10 }}</span>
			</Button>
			<Button
				size="lg"
				class="shadow-sm hover:shadow-md transition-all"
				:disabled="position.x === 0 && position.y === 0"
				@click="position = { x: 0, y: 0 }"
			>
				Center
			</Button>
		</div>
	</div>
	<div
		v-else
		class="size-full flex items-center justify-center text-balance text-center"
	>
		<p class="text-lg text-gray-500">
			Hit the compare button on any image.
		</p>
	</div>
</template>

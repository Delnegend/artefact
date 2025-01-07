<script setup lang="ts">
import { clamp } from "@vueuse/core";
import { ChevronsLeftRight, Columns2, Search, SquareSplitHorizontal } from "lucide-vue-next";
import { ref, watch } from "vue";

import { imageCompareImages, imageCompareMode } from "~/composables/states";

import Button from "./ui/button/Button.vue";

const containerRef = ref<HTMLDivElement | null>(null);

// ===== for scaling the image =====

const scale = ref(1);
const MIN_SCALE = 0.1; // DO NOT PUT 0 OR SMALLER
const MAX_SCALE = 100;
function handleWheel(e: WheelEvent): void {
	const delta = e.deltaY > 0 ? 0.9 : 1.1;
	scale.value = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale.value * delta));
}

// ===== for moving the image =====

const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const position = ref({ x: 0, y: 0 });

function doDrag(e: MouseEvent | TouchEvent): void {
	if (!isDragging.value) { return; }
	requestAnimationFrame(() => {
		const pos = "touches" in e ? e.touches[0] : e;
		position.value = {
			x: (pos.clientX - dragStart.value.x) / scale.value,
			y: (pos.clientY - dragStart.value.y) / scale.value,
		};
	});
}

function stopDrag(): void {
	isDragging.value = false;

	document.removeEventListener("mousemove", doDrag);
	document.removeEventListener("mouseup", stopDrag);
	document.removeEventListener("touchmove", doDrag);
	document.removeEventListener("touchend", stopDrag);
}

function startDrag(e: MouseEvent | TouchEvent): void {
	isDragging.value = true;

	const pos = "touches" in e ? e.touches[0] : e;
	dragStart.value = {
		x: pos.clientX - position.value.x * scale.value,
		y: pos.clientY - position.value.y * scale.value,
	};

	// capture mouse events globally
	document.addEventListener("mousemove", doDrag);
	document.addEventListener("mouseup", stopDrag);
	document.addEventListener("touchmove", doDrag);
	document.addEventListener("touchend", stopDrag);
}

// ===== for dragging the overlay handle =====

const isDraggingSlidingHandle = ref(false);
const slidingHandlePositionPercent = ref(50); // 0-100

function doDragSlidingHandle(e: MouseEvent | TouchEvent): void {
	if (!isDraggingSlidingHandle.value || !containerRef.value) { return; }

	requestAnimationFrame(() => {
		if (!containerRef.value) { return; }

		const pos = "touches" in e ? e.touches[0] : e;
		const rect = containerRef.value.getBoundingClientRect();
		const percentage = ((pos.clientX - rect.left) / rect.width) * 100;
		slidingHandlePositionPercent.value = clamp(percentage, 0, 100);
	});
}

function stopDragSlidingHandle(): void {
	isDraggingSlidingHandle.value = false;
	document.removeEventListener("mousemove", doDragSlidingHandle);
	document.removeEventListener("mouseup", stopDragSlidingHandle);
	document.removeEventListener("touchmove", doDragSlidingHandle);
	document.removeEventListener("touchend", stopDragSlidingHandle);
}

function startDragSlidingHandle(): void {
	isDraggingSlidingHandle.value = true;

	// capture mouse events globally
	document.addEventListener("mousemove", doDragSlidingHandle);
	document.addEventListener("mouseup", stopDragSlidingHandle);
	document.addEventListener("touchmove", doDragSlidingHandle);
	document.addEventListener("touchend", stopDragSlidingHandle);
}

// reset scale and position when image changes
watch(imageCompareImages, () => {
	scale.value = 1;
	position.value = { x: 0, y: 0 };
});
</script>

<template>
	<div
		v-if="imageCompareImages.jpegBlobUrl && imageCompareImages.pngBlobUrl"
		ref="containerRef"
		class="grid size-full relative"
		:style="{
			display: imageCompareMode === 'side-by-side' ? 'grid' : undefined,
			gridTemplateColumns: imageCompareMode === 'side-by-side' ? 'repeat(2, minmax(0, 1fr))' : undefined,
		}"
		@wheel.prevent="handleWheel">

		<!-- second/png image -->
		<div
			class="flex justify-center items-center overflow-hidden cursor-grab active:cursor-grabbing order-1"
			:class="{
				'size-full': imageCompareMode === 'side-by-side',
				'size-full absolute top-0 left-0': imageCompareMode === 'overlay',
			}"
			:style="{
				position: imageCompareMode === 'side-by-side' ? 'relative' : 'absolute',
			}"

			@mousedown.prevent="startDrag"
			@touchstart="startDrag">
			<img
				:src="imageCompareImages.pngBlobUrl"
				class="size-auto max-w-none select-none origin-center"
				:style="{
					transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
					transition: isDragging ? 'none' : 'transform 0.1s ease-out',
					imageRendering: 'pixelated',
				}">
		</div>

		<!-- first/jpeg image -->
		<div
			class="cursor-grab overflow-hidden flex items-center active:cursor-grabbing -order-1"
			:class="{
				'justify-center size-full': imageCompareMode === 'side-by-side',
				'justify-center h-full w-full absolute top-0 left-0': imageCompareMode === 'overlay',
			}"
			:style="{
				clipPath: imageCompareMode === 'overlay' ? `inset(0 ${100 - slidingHandlePositionPercent}% 0 0)` : undefined,
			}"

			@mousedown.prevent="startDrag"
			@touchstart="startDrag">
			<img
				:src="imageCompareImages.jpegBlobUrl"
				class="size-auto max-w-none select-none origin-center"
				:style="{
					transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
					transition: isDragging ? 'none' : 'transform 0.1s ease-out',
					imageRendering: 'pixelated',
				}"
				@wheel.prevent="handleWheel">
		</div>

		<!-- compare sliding handle -->
		<div
			v-if="imageCompareMode === 'overlay'"
			class="absolute top-0 -translate-x-1/2 w-[2px] bg-secondary h-full flex items-center justify-center cursor-grab active:cursor-grabbing"
			:style="{ left: `${slidingHandlePositionPercent}%` }"
			@mousedown.prevent="startDragSlidingHandle"
			@touchstart="startDragSlidingHandle">
			<ChevronsLeftRight
				:size="50"
				color="hsl(var(--primary))"
				class="p-2 shadow-lg bg-primary-foreground overflow-visible rounded-full" />
		</div>

		<!-- control buttons, bottom left -->
		<div
			class="flex absolute bottom-4 left-5 backdrop-blur bg-black/60 rounded-md">
				<Button
					size="lg"
					variant="secondary"
					class="p-4 shadow-sm hover:shadow-md transition-all"
					:disabled="scale === 1 && position.x === 0 && position.y === 0"
					@click="{ scale = 1; position = { x: 0, y: 0 }; }">
					<Search />
					<span class="w-7">x{{ Math.round(scale * 10) / 10 }}</span>
				</Button>
			</div>

		<!-- control buttons, bottom right -->
		<div
			class="flex absolute bottom-4 right-5 backdrop-blur bg-black/60 rounded-md gap-[1px]">
				<Button
					size="lg"
					variant="secondary"
					class="shadow-sm hover:shadow-md rounded-r-none p-4"
					:disabled="imageCompareMode === 'side-by-side'"
					@click="imageCompareMode = 'side-by-side'">
					<Columns2 />
				</Button>
				<Button
					size="lg"
					variant="secondary"
					class="shadow-sm hover:shadow-md rounded-l-none p-4"
					:disabled="imageCompareMode === 'overlay'"
					@click="imageCompareMode = 'overlay'">
					<SquareSplitHorizontal />
				</Button>
			</div>

		<!-- collapse image input btn -->
		<Button
			class="absolute top-4 left-5 aspect-square p-0"
			variant="secondary"
			size="lg"
			@click="$emit('toggle-image-input-panel')">
			<PanelLeftOpen v-if="imageInputPanelRef?.isCollapsed && displayMode === 'horizontal'" />
			<PanelLeftClose v-if="!imageInputPanelRef?.isCollapsed && displayMode === 'horizontal'" />
			<PanelTopOpen v-if="imageInputPanelRef?.isCollapsed && displayMode === 'vertical'" />
			<PanelTopClose v-if="!imageInputPanelRef?.isCollapsed && displayMode === 'vertical'" />
		</Button>
	</div>

	<div
		v-else
		class="size-full flex items-center justify-center text-balance text-center">
		<p class="text-lg text-gray-500">
			Hit the compare button on any image.
		</p>
	</div>
</template>

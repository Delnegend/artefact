<script setup lang="ts">
import { clamp } from "@vueuse/core";
import { ChevronsLeftRight, ChevronUp, Columns2, PanelLeftClose, PanelLeftOpen, PanelTopClose, PanelTopOpen, SquareSplitHorizontal } from "lucide-vue-next";
import { onMounted, onUnmounted, ref, watch } from "vue";

import Button from "~/components/ui/button/Button.vue";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from "~/components/ui/dropdown-menu";
import { displayMode, imageInputPanelRef, useImageCompareStore } from "~/composables";

const imageCompareStore = useImageCompareStore();

defineEmits(["toggle-image-input-panel"]);

const containerRef = ref<HTMLDivElement | null>(null);

// ===== for scaling the image =====

const scale = ref(1);
const MIN_SCALE = 0.1; // DO NOT PUT 0 OR SMALLER
const MAX_SCALE = 100;
function handleWheel(e: WheelEvent): void {
	const delta = e.deltaY > 0 ? 0.9 : 1.1;
	scale.value = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale.value * delta));
}

// ===== for scaling the image but on mobile =====
let initialDistance = 0;
let rafId: number | null = null;
let initialScale = 1;
// eslint-disable-next-line no-unused-vars
let touchMoveHandler: ((e: TouchEvent)=> void) | null = null;

function updateScaleMobile(newDistance: number): void {
	if (rafId !== null) { cancelAnimationFrame(rafId); }
	rafId = requestAnimationFrame((): void => {
		scale.value = clamp(initialScale * (newDistance / initialDistance), MIN_SCALE, MAX_SCALE);
	});
}

function cleanup(): void {
	if (touchMoveHandler) {
		document.removeEventListener("touchmove", touchMoveHandler);
		touchMoveHandler = null;
	}
	if (rafId !== null) {
		cancelAnimationFrame(rafId);
		rafId = null;
	}
}

function handleTouchStart(e: TouchEvent): void {
	if (e.touches.length !== 2) { return; }

	const touch1 = e.touches[0];
	const touch2 = e.touches[1];
	initialDistance = Math.hypot(touch1.clientX - touch2.clientX, touch1.clientY - touch2.clientY);
	initialScale = scale.value;

	function touchMoveHandler(e: TouchEvent): void {
		if (e.touches.length !== 2) { return; }

		const touch1 = e.touches[0];
		const touch2 = e.touches[1];
		const newDistance = Math.hypot(touch1.clientX - touch2.clientX, touch1.clientY - touch2.clientY);
		updateScaleMobile(newDistance);
	}

	document.addEventListener("touchmove", touchMoveHandler, { passive: true });
	document.addEventListener("touchend", cleanup, { once: true });
}

onMounted(() => {
	document.addEventListener("touchstart", handleTouchStart, { passive: false });
});

onUnmounted(() => {
	document.removeEventListener("touchstart", handleTouchStart);
	cleanup();
});

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
watch(() => [imageCompareStore.jpegBlobUrl, imageCompareStore.outputImgBlobUrl], () => {
	scale.value = 1;
	position.value = { x: 0, y: 0 };
});
</script>

<template>
	<div
		v-if="imageCompareStore.jpegBlobUrl && imageCompareStore.outputImgBlobUrl"
		ref="containerRef"
		class="size-full relative"
		:class="{ 'grid grid-cols-2 gap-[2px]': imageCompareStore.compareMode === 'side-by-side' }"
		@wheel.prevent="handleWheel">

		<!-- second/processed image -->
		<div
			class="flex size-full justify-center items-center overflow-hidden cursor-grab active:cursor-grabbing order-1"
			:class="{ 'absolute top-0 left-0': imageCompareStore.compareMode === 'overlay' }"

			@mousedown.prevent="startDrag"
			@touchstart="startDrag">
			<img
				:src="imageCompareStore.outputImgBlobUrl"
				class="size-auto max-w-none select-none origin-center"
				:style="{
					transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
					transition: isDragging ? 'none' : 'transform 0.1s ease-out',
					imageRendering: 'pixelated',
				}">
		</div>

		<!-- first/jpeg image -->
		<div
			class="justify-center size-full cursor-grab overflow-hidden flex items-center active:cursor-grabbing -order-1"
			:class="{ 'absolute top-0 left-0': imageCompareStore.compareMode === 'overlay' }"
			:style="{ clipPath: imageCompareStore.compareMode === 'overlay' ? `inset(0 ${100 - slidingHandlePositionPercent}% 0 0)` : undefined }"

			@mousedown.prevent="startDrag"
			@touchstart="startDrag">
			<img
				:src="imageCompareStore.jpegBlobUrl"
				class="size-auto max-w-none select-none origin-center"
				:style="{
					transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
					transition: isDragging ? 'none' : 'transform 0.1s ease-out',
					imageRendering: 'pixelated',
				}">
		</div>

		<!-- compare sliding handle -->
		<div
			v-if="imageCompareStore.compareMode === 'overlay'"
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
			class="flex absolute bottom-4 left-5 backdrop-blur bg-secondary/60 rounded-md">
			<DropdownMenu>
				<DropdownMenuTrigger>
					<div class="px-3">
						<ChevronUp class="translate-x-[1px]" />
					</div>
				</DropdownMenuTrigger>
				<DropdownMenuContent>
					<DropdownMenuItem
						v-for="n in [20, 10, 5, 2, 1]"
						:key="n"
						class="h-12 font-mono flex justify-center"
						@click="scale = n">
						x{{ n }}
						<DropdownMenuSeparator />
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<Button
				size="lg"
				variant="secondary"
				class="aspect-square px-4 shadow-sm hover:shadow-md transition-all font-mono"
				:disabled="scale === 1 && position.x === 0 && position.y === 0"
				@click="{ scale = 1; position = { x: 0, y: 0 }; }">
				x{{ Math.round(scale * 10) / 10 }}
			</Button>
		</div>

		<!-- control buttons, bottom right -->
		<div
			class="flex absolute bottom-4 right-5 backdrop-blur bg-primary-foreground/60 rounded-md gap-[1px] z-10">
			<Button
				size="lg"
				variant="secondary"
				class="shadow-sm hover:shadow-md rounded-r-none p-4"
				:disabled="imageCompareStore.compareMode === 'side-by-side'"
				@click="imageCompareStore.compareMode = 'side-by-side'">
				<Columns2 />
			</Button>
			<Button
				size="lg"
				variant="secondary"
				class="shadow-sm hover:shadow-md rounded-l-none p-4"
				:disabled="imageCompareStore.compareMode === 'overlay'"
				@click="imageCompareStore.compareMode = 'overlay'">
				<SquareSplitHorizontal />
			</Button>
		</div>

		<!-- collapse image input btn -->
		<Button
			class="absolute top-4 left-5 aspect-square p-0 backdrop-blur"
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

<script setup lang="ts">
import { onMounted } from "vue";

import ActionButtons from "./components/ActionButtons.vue";
import AppHeader from "./components/AppHeader.vue";
import ImageCompare from "./components/ImageCompare.vue";
import ImageInput from "./components/ImageInput.vue";
import ImageList from "./components/ImageList.vue";
import { displayMode, useImageCompareStore } from "./composables/states";

const imageCompareStore = useImageCompareStore();

function toggleImageInputPanel(): void {
	if (!imageCompareStore.imageInputPanelRef) { return; }
	const imageInputPanel = document.querySelector<HTMLDivElement>(".image-input-panel");
	if (!imageInputPanel) { return; }

	imageInputPanel.style.transition = "flex 150ms cubic-bezier(0.4, 0, 0.2, 1)";
	if (imageCompareStore.imageInputPanelRef.isCollapsed) {
		imageCompareStore.imageInputPanelRef.expand();
	} else {
		imageCompareStore.imageInputPanelRef.collapse();
	}
	setTimeout(() => {
		imageInputPanel.style.transition = "";
	}, 150);
}

onMounted(() => {
	if (window.innerWidth < window.innerHeight) {
		displayMode.value = "vertical";
	}

	setTimeout(() => {
		if (!imageCompareStore.imageInputPanelRef) { return; }
		if (imageCompareStore.imageInputPanelRef.isCollapsed) {
			imageCompareStore.imageInputPanelRef.expand();
		}
	}, 0);
});

</script>

<template>
	<div class="max-h-dvh h-dvh w-full">
		<AppHeader />
		<Toaster />

		<ResizablePanelGroup
			:direction="displayMode"
			class="h-full max-h-[calc(100vh-4rem)]"
			auto-save-id="app-layout">

			<ResizablePanel
				ref="imageInputPanelRef"
				collapsible
				:collapsed-size="0"
				:default-size="20"
				class="image-input-panel">
				<div
					class="grid grid-rows-[auto,1fr,auto] h-full"
					:style="{
						'min-width': displayMode === 'horizontal' ? '320px' : 0,
						'min-height': displayMode === 'vertical' ? '370px' : undefined,
					}">
					<ImageInput />
					<ImageList />
					<ActionButtons />
				</div>
			</ResizablePanel>

			<ResizableHandle with-handle />

			<ResizablePanel>
				<div
					ref="imageCompareContainerRef"
					class="size-full">
					<ImageCompare @toggle-image-input-panel="toggleImageInputPanel" />
				</div>
			</ResizablePanel>
		</ResizablePanelGroup>
	</div>
</template>

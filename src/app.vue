<script setup lang="ts">
import { onMounted } from "vue";

import ActionButtons from "./components/ActionButtons.vue";
import AppHeader from "./components/AppHeader.vue";
import ImageCompare from "./components/ImageCompare.vue";
import ImageInput from "./components/ImageInput.vue";
import ImageList from "./components/ImageList.vue";
import { displayMode, imageInputPanelRef } from "./composables/states";

const lefPaneWidth = ((): number => {
	const storedWidth = Number(localStorage.getItem("image-input-panel-size"));
	if (!Number.isNaN(storedWidth)) {
		return storedWidth;
	}
	return 20;
})();

function handlePanelResize(): void {
	const flexVal = document.querySelector(".image-input-panel")?.computedStyleMap()
		.get("flex")
		?.toString();
	localStorage.setItem("image-input-panel-size", flexVal ?? lefPaneWidth.toString());
}

function toggleImageInputPanel(): void {
	if (!imageInputPanelRef.value) { return; }
	const imageInputPanel = document.querySelector<HTMLDivElement>(".image-input-panel");
	if (!imageInputPanel) { return; }

	imageInputPanel.style.transition = "flex 150ms cubic-bezier(0.4, 0, 0.2, 1)";
	if (imageInputPanelRef.value.isCollapsed) {
		imageInputPanelRef.value.expand();
	} else {
		imageInputPanelRef.value.collapse();
	}
	setTimeout(() => {
		imageInputPanel.style.transition = "";
	}, 150);
}

onMounted(() => {
	if ((/Mobi/iu).test(window.navigator.userAgent)) {
		displayMode.value = "vertical";
	}
});
</script>

<template>
	<div class="max-h-dvh h-dvh w-full">
		<AppHeader />
		<Toaster />

		<ResizablePanelGroup
			:direction="displayMode"
			class="h-full max-h-[calc(100vh-4rem)]">

			<ResizablePanel
				ref="imageInputPanelRef"
				collapsible
				:collapsed-size="0"
				:default-size="lefPaneWidth"
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

			<ResizableHandle
				with-handle
				@dragging="handlePanelResize" />

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

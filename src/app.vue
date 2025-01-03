<script setup lang="ts">
import ActionButtons from "./components/ActionButtons.vue";
import AppHeader from "./components/AppHeader.vue";
import ImageCompare from "./components/ImageCompare.vue";
import ImageInput from "./components/ImageInput.vue";
import ImageList from "./components/ImageList.vue";
import { displayMode } from "./composables/states";

const lefPaneWidth = ((): number => {
	const storedWidth = Number(localStorage.getItem("image-input-panel-width"));
	if (!Number.isNaN(storedWidth)) {
		return storedWidth;
	}
	return 20;
})();

function handlePanelResize(): void {
	const leftPanel = document.querySelector(".left-panel");
	if (!leftPanel) { return; }
	localStorage.setItem("image-input-panel-width", leftPanel.getAttribute("data-panel-size") ?? lefPaneWidth.toString());
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
			class="h-full max-h-[calc(100vh-4rem)]"
		>
			<ResizablePanel
				:default-size="lefPaneWidth"
				class="left-panel grid grid-rows-[auto,1fr,auto]"
				:style="{
					'min-width': displayMode === 'horizontal' ? '320px' : 0,
					'min-height': displayMode === 'vertical' ? '370px' : undefined,
				}"
			>
				<ImageInput />
				<ImageList />
				<ActionButtons />
			</ResizablePanel>

			<ResizableHandle
				with-handle
				@dragging="handlePanelResize"
			/>

			<ResizablePanel>
				<div
					ref="imageCompareContainerRef"
					class="size-full"
				>
					<ImageCompare />
				</div>
			</ResizablePanel>
		</ResizablePanelGroup>
	</div>
</template>

<script setup lang="ts">
import AppHeader from './components/AppHeader.vue'
import ImageCompare from './components/ImageCompare.vue'
import ImageInput from './components/ImageInput.vue'
import ImageList from './components/ImageList.vue'
import {
	ResizableHandle,
	ResizablePanel,
	ResizablePanelGroup
} from './components/ui/resizable'
import { Toaster } from './components/ui/sonner'
import { displayMode, imageInputPanelRef } from './composables'
import { onMounted } from 'vue'

function toggleImageInputPanel(): void {
	if (!imageInputPanelRef.value) return

	const imageInputPanel =
		document.querySelector<HTMLDivElement>('.image-input-panel')
	if (!imageInputPanel) return

	imageInputPanel.style.transition = 'flex 150ms cubic-bezier(0.4, 0, 0.2, 1)'
	if (imageInputPanelRef.value.isCollapsed) imageInputPanelRef.value.expand()
	else imageInputPanelRef.value.collapse()

	window.setTimeout(() => (imageInputPanel.style.transition = ''), 150)
}

onMounted(() => {
	if (window.innerWidth < window.innerHeight) displayMode.value = 'vertical'

	window.setTimeout(() => {
		if (!imageInputPanelRef.value) return

		if (imageInputPanelRef.value.isCollapsed)
			imageInputPanelRef.value.expand()
	}, 0)
})
</script>

<template>
	<div class="h-dvh max-h-dvh w-full">
		<Toaster />
		<AppHeader />
		<NuxtPwaManifest />
		<ResizablePanelGroup
			:direction="displayMode"
			class="h-full max-h-[calc(100vh-4rem)]"
			auto-save-id="app-layout"
		>
			<ResizablePanel
				ref="imageInputPanelRef"
				collapsible
				:collapsed-size="0"
				:default-size="20"
				class="image-input-panel"
			>
				<div
					class="grid h-full grid-rows-[auto_1fr_auto]"
					:style="{
						'min-width': displayMode === 'horizontal' ? '320px' : 0,
						'min-height':
							displayMode === 'vertical' ? '370px' : undefined
					}"
				>
					<ImageInput />
					<ImageList />
					<!-- <ActionButtons /> -->
				</div>
			</ResizablePanel>

			<ResizableHandle with-handle />

			<ResizablePanel>
				<div ref="imageCompareContainerRef" class="size-full">
					<ImageCompare
						@toggle-image-input-panel="toggleImageInputPanel"
					/>
				</div>
			</ResizablePanel>
		</ResizablePanelGroup>
	</div>
</template>

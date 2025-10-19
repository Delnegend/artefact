<script setup lang="ts">
import { buttonBaseClassTw, buttonVariantsTw } from './ui/button'
import { useFileDialog } from '@vueuse/core'
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import { imageListStoreOps } from '~/composables/use-image-list-store'
import { cn } from '~/utils/cn'

async function handleIncomingFiles(files: FileList | null): Promise<void> {
	if (!files) return

	try {
		await imageListStoreOps.addFileList(files)
	} catch (error) {
		toast.error('Failed to process files', {
			description: `${error}`
		})
	}
}

// Drag and drop
const nothingOver = ref(true)
function handleOnDrop(event: DragEvent): void {
	nothingOver.value = true
	const files = event.dataTransfer?.files
	if (!files) return

	for (const file of files) {
		if (file.type !== 'image/jpeg') {
			toast.error('Only JPEG files are supported', {
				description: `File ${file.name} is not a JPEG file`
			})
			return
		}
	}
	void handleIncomingFiles(files)
}

// Click to select
const fileDialog = useFileDialog({ accept: 'image/jpeg' })
fileDialog.onChange(async (files) => {
	await handleIncomingFiles(files)
})
</script>

<template>
	<div class="relative">
		<input
			id="image-input"
			class="absolute top-0 left-0 m-4 h-28 w-[calc(100%-2rem)] rounded-md bg-transparent"
			aria-label="image-input"
			@click.prevent="fileDialog.open()"
		/>

		<label
			for="image-input"
			:class="
				cn(
					buttonBaseClassTw,
					buttonVariantsTw.secondary,
					'focus:ring-primary-500 focus:ring-opacity-50 absolute top-0 left-0 m-4 flex h-28 w-[calc(100%-2rem)] flex-col border border-dashed border-neutral-300 px-4 py-2 text-center text-xl text-balance select-none focus:ring-2 focus:outline-hidden',
					!nothingOver ? 'bg-secondary/80' : ''
				)
			"
			@dragover.prevent="nothingOver = false"
			@drop.prevent="handleOnDrop"
			@dragleave.prevent="nothingOver = true"
		>
			{{
				nothingOver
					? 'Drag JPEG files here or click to select'
					: 'Drop here'
			}}
		</label>

		<div class="m-4 h-28" />
	</div>
</template>

<script setup lang="ts">
import { useFileDialog } from "@vueuse/core";
import { ref } from "vue";
import { toast } from "vue-sonner";

import { useImageDisplayListStore } from "~/composables/use-image-display-list-store";
import { cn } from "~/utils/cn";

import { buttonBaseClassTw, buttonVariantsTw } from "./ui/button";

const imageDisplayListStore = useImageDisplayListStore();

async function handleIncomingFiles(files: FileList | null): Promise<void> {
	if (!files) { return; }
	try {
		await imageDisplayListStore.addFileList(files);
	} catch (error) {
		toast.error("Failed to process files", {
			description: `${error}`,
		});
	}
}

// Drag and drop
const nothingOver = ref(true);
function handleOnDrop(event: DragEvent): void {
	nothingOver.value = true;
	const files = event.dataTransfer?.files;
	if (!files) { return; }
	for (const file of files) {
		if (file.type !== "image/jpeg") {
			toast.error("Only JPEG files are supported", {
				description: `File ${file.name} is not a JPEG file`,
			});
			return;
		}
	}
	void handleIncomingFiles(files);
}

// Click to select
const fileDialog = useFileDialog({ accept: "image/jpeg" });
fileDialog.onChange(async (files) => { await handleIncomingFiles(files); });
</script>

<template>
	<div class="relative">
		<input
			id="image-input"
			class="absolute left-0 top-0 bg-transparent w-[calc(100%-2rem)] h-28 m-4 rounded-md"
			aria-label="image-input"
			@click.prevent="fileDialog.open()">

		<label
			for="image-input"
			:class="cn(buttonBaseClassTw, buttonVariantsTw.secondary, 'h-28 border w-[calc(100%-2rem)] flex flex-col m-4 px-4 py-2 text-balance text-xl border-neutral-300 border-dashed text-center focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-opacity-50 left-0 top-0 absolute select-none', !nothingOver ? 'bg-secondary/80' : '')"
			@dragover.prevent="nothingOver = false;"
			@drop.prevent="handleOnDrop"
			@dragleave.prevent="nothingOver = true;">
			{{ nothingOver ? "Drag JPEG files here or click to select" : "Drop here" }}
		</label>

		<div class="h-28 m-4" />
	</div>
</template>

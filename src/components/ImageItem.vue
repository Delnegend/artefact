<script setup lang="ts">
import { toast } from "vue-sonner";

import { humanReadableSize } from "~/composables/human-readable-size";
import { db, imageCompare, imageDisplayList } from "~/composables/states";
import type { ImageItemForDisplay } from "~/composables/types";

const props = defineProps<{ jpegFileHash: string; info: ImageItemForDisplay }>();

const pngBlobUrl = ref<string | undefined>(props.info.pngBlobUrl);
const isProcessing = ref(false);

const worker = new Worker(
	new URL("~/composables/artefact-worker.ts", import.meta.url),
	{ type: "module" },
);

worker.onmessage = (e): void => {
	const { blobUrl, error } = e.data as { blobUrl?: string; error?: string };

	if (error) {
		toast.error("Error", {
			description: error,
		});
		isProcessing.value = false;
		return;
	}

	if (!blobUrl) {
		toast.error("Error", {
			description: "The worker doesn't return the image nor any error.",
		});
		isProcessing.value = false;
		return;
	}

	pngBlobUrl.value = blobUrl;
};

worker.onerror = (e): void => {
	toast.error("Error", {
		description: e.message,
	});
	isProcessing.value = false;
};

function process(): void {
	if (pngBlobUrl.value) {
		toast.error("Error", {
			description: "The image is already processed.",
		});
		return;
	}

	if (isProcessing.value) {
		toast.error("Error", {
			description: "The image is already being processed.",
		});
		return;
	}

	isProcessing.value = true;
	worker.postMessage(props.jpegFileHash);
}

function download(): void {
	if (!pngBlobUrl.value) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	const a = document.createElement("a");
	a.style.display = "none";
	a.download = `${props.info.name.split(".").slice(0, -1)
		.join(".")}.png`;
	a.href = pngBlobUrl.value;
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
}

function compare(): void {
	if (!pngBlobUrl.value) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	imageCompare.value = {
		jpegBlobUrl: props.info.jpegBlobUrl,
		pngBlobUrl: pngBlobUrl.value,
	};
}
</script>

<template>
	<div class="px-4 flex flex-col gap-3">
		<div class="grid grid-cols-[4rem,1fr,auto] h-fit gap-4 items-center">
			<img
				:src="info.jpegBlobUrl"
				class="rounded-md size-16 object-cover aspect-square"
			>

			<div class="flex flex-col h-full justify-between">
				<div class="font-medium line-clamp-1">
					{{ info.name }}
				</div>
				<div class="text-neutral-500 flex flex-col text-sm">
					<span class="-mb-1">{{ humanReadableSize(info.size) }}</span>
					<span>{{ new Date(info.dateAdded).toLocaleDateString("en-US", {
						year: "numeric",
						month: "short",
						day: "numeric",
					}) }}</span>
				</div>
			</div>
		</div>

		<div class="grid grid-cols-3 gap-2">
			<Button
				v-if="!pngBlobUrl"
				:disabled="isProcessing"
				@click="process"
			>
				Process
			</Button>
			<Button
				v-if="pngBlobUrl"
				variant="outline"
				:disabled="!pngBlobUrl"
				@click="download"
			>
				Download
			</Button>

			<Button
				variant="secondary"
				:disabled="!pngBlobUrl"
				@click="compare"
			>
				Compare
			</Button>

			<Button
				variant="destructive"
				@click="() => {
					worker.terminate();
					delete imageDisplayList.value[jpegFileHash];
					void db.delete('files', jpegFileHash);
				}"
			>
				<div>Remove</div>
			</Button>
		</div>
	</div>
</template>

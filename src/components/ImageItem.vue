<script setup lang="ts">
import { LoaderCircle } from "lucide-vue-next";
import { h, ref } from "vue";
import { toast } from "vue-sonner";

import { humanReadableSize } from "~/composables/human-readable-size";
import { db, imageCompareImages, imageDisplayList } from "~/composables/states";
import type { ImageItemForDisplay } from "~/composables/types";

const props = defineProps<{ jpegFileHash: string; info: ImageItemForDisplay }>();

const pngBlobUrl = ref<string | undefined>(props.info.pngBlobUrl);
const isProcessing = ref(false);

const worker = new Worker(
	new URL("~/composables/artefact-worker.ts", import.meta.url),
	{ type: "module", name: "artefact-worker" },
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

	toast.info("Success", {
		description: h("div", [
			h("code", props.info.name),
			" is processed successfully.",
		]),
	});
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

	imageCompareImages.value = {
		jpegBlobUrl: props.info.jpegBlobUrl,
		pngBlobUrl: pngBlobUrl.value,
	};
}

function remove(): void {
	worker.terminate();
	imageDisplayList.value.delete(props.jpegFileHash);
	void db.delete("files", props.jpegFileHash);
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
					<span class="-mb-1">{{ humanReadableSize(info.size) }} | {{ info.width }}x{{ info.height }}</span>
					<span>{{ new Date(info.dateAdded).toLocaleTimeString("en-US", {
						year: "numeric",
						month: "short",
						day: "numeric",
						hour: "numeric",
						minute: "numeric",
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
				<span v-if="!isProcessing">Process</span>
				<span v-else class="animate-spin"><LoaderCircle /></span>
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
				@click="remove"
			>
				<div>Remove</div>
			</Button>
		</div>
	</div>
</template>

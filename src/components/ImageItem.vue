<script setup lang="ts">
import { LoaderCircle } from "lucide-vue-next";
import { h, ref } from "vue";
import { toast } from "vue-sonner";

import Badge from "~/components/ui/badge/Badge.vue";
import { humanReadableSize } from "~/composables/human-readable-size";
import { db, imageDisplayList, useImageCompareStore, useProcessingConfig } from "~/composables/states";
import type { ImageItemForDisplay, OutputImgFormat, WorkerInput, WorkerOutput } from "~/composables/types";
import { cn } from "~/lib/utils";

const props = defineProps<{
	jpegFileHash: string;
	info: ImageItemForDisplay;
	class?: string;
}>();

const imageCompareStore = useImageCompareStore();
const processingConfig = useProcessingConfig();

const outputImgBlobUrl = ref<string | undefined>(props.info.outputImgBlobUrl);
const outputImgFormat = ref<OutputImgFormat | undefined>(props.info.outputImgFormat);

const isProcessing = ref(false);

const worker = new Worker(
	new URL("~/composables/artefact-worker.ts", import.meta.url),
	{ type: "module", name: "artefact-worker" },
);

worker.onmessage = (e): void => {
	const { blobUrl, error, timeTakenInMs, outputFormat } = e.data as WorkerOutput;

	if (error) {
		toast.error("Error", {
			description: error,
		});
		isProcessing.value = false;
		return;
	}

	if (!blobUrl || timeTakenInMs === undefined) {
		toast.error("Error", {
			description: "The worker doesn't return the image nor any error.",
		});
		isProcessing.value = false;
		return;
	}

	toast.info("Success", {
		description: h("div", [
			h("code", props.info.name),
			" done in ",
			h("code", `${(timeTakenInMs / 1000).toFixed(2)}s`),
		]),
	});
	outputImgBlobUrl.value = blobUrl;
	outputImgFormat.value = outputFormat;
};

worker.onerror = (e): void => {
	toast.error("Error", {
		description: e.message,
	});
	isProcessing.value = false;
};

function process(): void {
	if (outputImgBlobUrl.value) {
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

	const req: WorkerInput = {
		jpegFileHash: props.jpegFileHash,
		config: {
			outputFormat: processingConfig.outputFormat,
			iterations: processingConfig.iterations,
			weight: processingConfig.weight,
			pWeight: processingConfig.pWeight,
			separateComponents: processingConfig.separateComponents,
		},
	};
	worker.postMessage(req);
}

function download(): void {
	if (!outputImgBlobUrl.value) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	const a = document.createElement("a");
	a.style.display = "none";
	a.download = `${props.info.name.split(".").slice(0, -1)
		.join(".")}.${outputImgFormat.value}`;
	a.href = outputImgBlobUrl.value;
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
}

function compare(): void {
	if (!outputImgBlobUrl.value) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	imageCompareStore.jpegBlobUrl = props.info.jpegBlobUrl;
	imageCompareStore.outputImgBlobUrl = outputImgBlobUrl.value;
}

function remove(): void {
	worker.terminate();
	void db.delete("files", props.jpegFileHash);
	imageDisplayList.value.delete(props.jpegFileHash);
	if (imageCompareStore.jpegBlobUrl === props.info.jpegBlobUrl) {
		imageCompareStore.jpegBlobUrl = undefined;
		imageCompareStore.outputImgBlobUrl = undefined;
	}
}

function reprocess(): void {
	toast.info("This feature is not implemented yet.");
}
</script>

<template>
	<div :class="cn('px-4 flex flex-col gap-3', props.class)">
		<div class="grid grid-cols-[auto,1fr] gap-4 items-center">
			<img
				:src="info.jpegBlobUrl"
				class="rounded-md size-16 object-cover aspect-square">

			<div class="flex flex-col size-full justify-between overflow-hidden">
				<div
					class="font-medium text-transparent bg-clip-text line-clamp-1 text-ellipsis overflow-hidden"
					:style="{
						backgroundImage: 'linear-gradient(90deg, hsl(var(--primary)) 70%, transparent 100%)'
					}">
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

		<div class="grid grid-cols-2 grid-rows-2 gap-2">
			<div class="grid">
				<Button
					v-if="!outputImgBlobUrl"
					:disabled="isProcessing"
					@click="process">
					<span v-if="!isProcessing">Process</span>
					<span v-else class="animate-spin">
						<LoaderCircle />
					</span>
				</Button>
				<Button
					v-if="outputImgBlobUrl"
					:disabled="!outputImgBlobUrl"
					class="relative"
					@click="download">
					Download
					<Badge class="absolute -bottom-3 scale-90 backdrop-blur-sm bg-primary-foreground/90" variant="outline">{{ outputImgFormat }}</Badge>
				</Button>
			</div>

			<Button
				variant="outline"
				:disabled="!outputImgBlobUrl"
				@click="reprocess">
				Re-process
			</Button>

			<Button
				variant="secondary"
				:disabled="!outputImgBlobUrl"
				@click="compare">
				Compare
			</Button>

			<Button
				variant="outline"
				class="text-red-500 hover:text-red-600"
				@click="remove">
				<div>Remove</div>
			</Button>
		</div>
	</div>
</template>

<script setup lang="ts">
import { LoaderCircle } from "lucide-vue-next";
import { h, ref, watchEffect } from "vue";
import { toast } from "vue-sonner";

import Badge from "~/components/ui/badge/Badge.vue";
import { useImageCompareStore, useProcessConfigStore, useSimpleArtefactWorker } from "~/composables";
import { useImageDisplayListStore } from "~/composables/use-image-display-list-store";
import { cn } from "~/utils/cn";
import { humanReadableSize } from "~/utils/human-readable-size";
import type { ImageItemForDisplay } from "~/utils/types";

const props = defineProps<{
	jpegFileHash: string;
	info: ImageItemForDisplay;
	class?: string;
}>();

const imageDisplayListStore = useImageDisplayListStore();
const imageCompareStore = useImageCompareStore();
const processingConfig = useProcessConfigStore();

const {
	error,
	output,
	processing,
	process: startProcess,
	terminate,
} = useSimpleArtefactWorker({ config: processingConfig.allConfig, jpegFileHash: props.jpegFileHash });
const queued = ref(false);

watchEffect(() => {
	if (error.value === null || error.value === "") { return; }
	toast.error("Error", { description: error.value });
});

watchEffect(async () => {
	if (!output.value) { return; }

	toast.success("Success", {
		description: h("div", [h("code", props.info.name), " done in ", h("code", output.value.timeTaken)]),
	});

	imageDisplayListStore.setOutputImgBlobUrl(
		props.jpegFileHash,
		await imageDisplayListStore.getOutputImgBlobUrl(props.jpegFileHash),
	);
	imageDisplayListStore.setOutputImgFormat(
		props.jpegFileHash,
		output.value.outputFormat,
	);
});

function process(): void {
	if (props.info.outputImgBlobUrl) {
		toast.error("Error", {
			description: "The image is already processed.",
		});
		return;
	}

	if (processing.value) {
		toast.error("Error", {
			description: "The image is already being processed.",
		});
		return;
	}

	queued.value = true;
	startProcess();
}

function download(): void {
	if (!props.info.outputImgBlobUrl) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	const a = document.createElement("a");
	a.style.display = "none";
	a.download = `${props.info.name.split(".").slice(0, -1)
		.join(".")}.${props.info.outputImgFormat}`;
	a.href = props.info.outputImgBlobUrl;
	document.body.appendChild(a);
	a.click();
	document.body.removeChild(a);
}

function compare(): void {
	if (!props.info.outputImgBlobUrl) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	imageCompareStore.jpegBlobUrl = props.info.jpegBlobUrl;
	imageCompareStore.outputImgBlobUrl = props.info.outputImgBlobUrl;
}

async function remove(): Promise<void> {
	terminate();
	try {
		await imageDisplayListStore.remove(props.jpegFileHash);
	} catch (error) {
		toast.error("Failed to remove image from DB", {
			description: `${error}`,
		});
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
					v-if="!props.info.outputImgBlobUrl"
					:disabled="processing"
					@click="process">
					<span v-if="!processing">Process</span>
					<span v-else class="animate-spin">
						<LoaderCircle />
					</span>
				</Button>
				<Button
					v-if="props.info.outputImgBlobUrl"
					:disabled="!props.info.outputImgBlobUrl"
					class="relative"
					@click="download">
					Download
					<Badge
						class="absolute -bottom-3 scale-90 backdrop-blur-sm bg-primary-foreground/90"
						variant="outline">
						{{ props.info.outputImgFormat }}
					</Badge>
				</Button>
			</div>

			<Button
				variant="outline"
				:disabled="!props.info.outputImgBlobUrl"
				@click="reprocess">
				Re-process
			</Button>

			<Button
				variant="secondary"
				:disabled="!props.info.outputImgBlobUrl"
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

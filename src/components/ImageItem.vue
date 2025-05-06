<script setup lang="ts">
import { LoaderCircle } from "lucide-vue-next";
import { h, ref, watchEffect } from "vue";
import { toast } from "vue-sonner";
import Button from "./ui/button/Button.vue";
import Badge from "~/components/ui/badge/Badge.vue";
import { useImageCompareStore, useProcessConfigStore, useSimpleArtefactWorker } from "~/composables";
import { imageListStoreOps, useImageListStore } from "~/composables/use-image-list-store"
import { cn } from "~/utils/cn";
import { getFileInDb } from "~/utils/db";
import { humanReadableSize } from "~/utils/human-readable-size";
import type { ImageItemForDisplay } from "~/utils/types";

const props = defineProps<{
	jpegFileHash: string;
	info: ImageItemForDisplay;
	class?: string;
}>();

const imgList = useImageListStore();
const imgCompStore = useImageCompareStore();
const processingConfig = useProcessConfigStore();

const task = useSimpleArtefactWorker({
	config: { ...processingConfig.value },
	jpegFileHash: props.jpegFileHash,
});

watchEffect(() => {
	if (task.error.value === null || task.error.value === "") {
		return;
	}
	toast.error("Error", { description: task.error.value });
});

watchEffect(async () => {
	if (!task.output.value) {
		return;
	}

	toast.success("Success", {
		description: h("div", [
			h("code", props.info.name),
			" done in ",
			h("code", task.output.value.timeTaken),
		]),
	});

	// we need to update the image list store because worker can't access it, only the db

	const latestImgFromDb = await getFileInDb(props.jpegFileHash);

	imgList.value[props.jpegFileHash].outputImgBlobUrl = latestImgFromDb?.outputImgArrayBuffer
		? URL.createObjectURL(
			new Blob([latestImgFromDb.outputImgArrayBuffer], {
				type: `image/${latestImgFromDb.outputImgFormat}`,
			}),
		)
		: undefined;

	imgList.value[props.jpegFileHash].outputImgFormat = task.output.value.outputFormat;
});

function process(): void {
	if (props.info.outputImgBlobUrl) {
		toast.error("Error", {
			description: "The image is already processed.",
		});
		return;
	}

	if (task.processing.value) {
		toast.error("Error", {
			description: "The image is already being processed.",
		});
		return;
	}

	task.process();
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
	a.download = `${props.info.name
		.split(".")
		.slice(0, -1)
		.join(".")}.${props.info.outputImgFormat}`;
	a.href = props.info.outputImgBlobUrl;
	document.body.append(a);
	a.click();
	a.remove();
}

function compare(): void {
	if (!props.info.outputImgBlobUrl) {
		toast.error("Error", {
			description: "The image is not processed yet.",
		});
		return;
	}

	imgCompStore.value.jpegBlobUrl = props.info.jpegBlobUrl;
	imgCompStore.value.outputImgBlobUrl = props.info.outputImgBlobUrl;
}

async function remove(): Promise<void> {
	task.terminate();
	try {
		imageListStoreOps.remove(props.jpegFileHash);
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
		<div class="grid grid-cols-[auto,1fr] items-center gap-4">
			<img
				:src="info.jpegBlobUrl"
				class="aspect-square size-16 rounded-md object-cover">

			<div class="flex size-full flex-col justify-between overflow-hidden">
				<div
					class="line-clamp-1 overflow-hidden text-ellipsis bg-clip-text font-medium text-transparent"
					:style="{
						backgroundImage: 'linear-gradient(90deg, hsl(var(--primary)) 70%, transparent 100%)'
					}">
					{{ info.name }}
				</div>

				<div class="flex flex-col text-sm text-neutral-500">
					<span class="-mb-1">
						{{ humanReadableSize(info.size) }} | {{ info.width }}x{{ info.height }}
					</span>
					<span>{{
						new Date(info.dateAdded).toLocaleTimeString("en-US", {
							year: "numeric",
							month: "short",
							day: "numeric",
							hour: "numeric",
							minute: "numeric"
						})
					}}</span>
				</div>
			</div>
		</div>

		<div class="grid grid-cols-2 grid-rows-2 gap-2">
			<div class="grid">
				<Button
					v-if="!props.info.outputImgBlobUrl"
					:disabled="task.processing.value"
					@click="process">
					<span v-if="!task.processing.value">Process</span>
					<span
						v-else
						class="animate-spin">
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
						class="absolute -bottom-3 scale-90 bg-primary-foreground/90 backdrop-blur-sm"
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

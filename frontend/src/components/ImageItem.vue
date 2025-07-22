<script setup lang="ts">
import Button from './ui/button/Button.vue'
import { LoaderCircle } from 'lucide-vue-next'
import { h, ref } from 'vue'
import { toast } from 'vue-sonner'
import Badge from '~/components/ui/badge/Badge.vue'
import { useImageCompareStore } from '~/composables/use-image-compare-store'
import {
	imageListStoreOps,
	useImageListStore
} from '~/composables/use-image-list-store'
import { useProcessConfigStore } from '~/composables/use-process-config-store'
import { cn } from '~/utils/cn'
import { getFileInDb } from '~/utils/db'
import { humanReadableSize } from '~/utils/human-readable-size'
import { runArtefactWorker } from '~/utils/run-artefact-worker'
import type { ImageItemForDisplay } from '~/utils/types'

const props = defineProps<{
	jpegFileHash: string
	info: ImageItemForDisplay
	class?: string
}>()

const imgList = useImageListStore()
const imgCompStore = useImageCompareStore()
const processingConfig = useProcessConfigStore()
const processing = ref(false)

function process(): void {
	if (props.info.outputImgBlobUrl) {
		toast.error('Error', {
			description: 'The image is already processed.'
		})
		return
	}

	if (processing.value) {
		toast.error('Error', {
			description: 'The image is already being processed.'
		})
		return
	}

	processing.value = true

	void (async (): Promise<void> => {
		const result = await runArtefactWorker({
			config: { ...processingConfig.value },
			jpegFileHash: props.jpegFileHash
		})

		const resultImgFromDb = await getFileInDb(props.jpegFileHash)
		processing.value = false

		if (result.type === 'error') {
			toast.error('Error', {
				description: `Can't process image ${resultImgFromDb?.jpegFileName}: ${result.error}`
			})
			return
		}

		toast.success('Success', {
			description: h('div', [
				h('code', props.info.name),
				' done in ',
				h('code', result.timeTaken)
			])
		})

		// we need to update the image list store because worker can't access it, only the db
		const targetImg = imgList.value[props.jpegFileHash]
		if (!targetImg) return
		targetImg.outputImgBlobUrl = resultImgFromDb?.outputImgArrayBuffer
			? URL.createObjectURL(
					new Blob([resultImgFromDb.outputImgArrayBuffer], {
						type: `image/${resultImgFromDb.outputImgFormat}`
					})
				)
			: undefined

		targetImg.outputImgFormat = result.outputFormat
	})()
}

function download(): void {
	if (!props.info.outputImgBlobUrl) {
		toast.error('Error', {
			description: 'The image is not processed yet.'
		})
		return
	}

	const a = document.createElement('a')
	a.style.display = 'none'
	a.download = `${props.info.name.split('.').slice(0, -1).join('.')}.${
		props.info.outputImgFormat
	}`
	a.href = props.info.outputImgBlobUrl
	document.body.append(a)
	a.click()
	a.remove()
}

function compare(): void {
	if (!props.info.outputImgBlobUrl) {
		toast.error('Error', {
			description: 'The image is not processed yet.'
		})
		return
	}

	imgCompStore.value.jpegBlobUrl = props.info.jpegBlobUrl
	imgCompStore.value.outputImgBlobUrl = props.info.outputImgBlobUrl
}

function remove(): void {
	if (processing.value) {
		toast.error('Error', {
			description: 'The image is already being processed.'
		})
		return
	}

	imageListStoreOps.remove(props.jpegFileHash)
}
</script>

<template>
	<div :class="cn('px-4 flex flex-col gap-3', props.class)">
		<div class="grid grid-cols-[auto_1fr] items-center gap-4">
			<img
				:src="info.jpegBlobUrl"
				class="aspect-square size-16 rounded-md object-cover"
			/>

			<div
				class="flex size-full flex-col justify-between overflow-hidden"
			>
				<div
					class="line-clamp-1 overflow-hidden text-ellipsis bg-clip-text font-medium text-transparent"
					:style="{
						backgroundImage:
							'linear-gradient(90deg, var(--primary) 70%, transparent 100%)'
					}"
				>
					{{ info.name }}
				</div>

				<div class="flex flex-col text-sm text-neutral-500">
					<span class="-mb-1">
						{{ humanReadableSize(info.size) }} | {{ info.width }}x{{
							info.height
						}}
					</span>
					<span>{{
						new Date(info.dateAdded).toLocaleTimeString('en-US', {
							year: 'numeric',
							month: 'short',
							day: 'numeric',
							hour: 'numeric',
							minute: 'numeric'
						})
					}}</span>
				</div>
			</div>
		</div>

		<div class="grid grid-cols-2 grid-rows-2 gap-2">
			<div class="grid">
				<Button
					v-if="!props.info.outputImgBlobUrl"
					:disabled="processing"
					@click="process"
				>
					<span v-if="!processing">Process</span>
					<span v-else class="animate-spin">
						<LoaderCircle />
					</span>
				</Button>
				<Button
					v-if="props.info.outputImgBlobUrl"
					:disabled="!props.info.outputImgBlobUrl"
					class="relative"
					@click="download"
				>
					Download
					<Badge
						class="absolute -bottom-3 scale-90 bg-primary-foreground/90 backdrop-blur-xs"
						variant="outline"
					>
						{{ props.info.outputImgFormat }}
					</Badge>
				</Button>
			</div>

			<Button variant="outline" :disabled="processing" @click="remove">
				Remove
			</Button>

			<Button
				class="col-span-2"
				variant="secondary"
				:disabled="!props.info.outputImgBlobUrl"
				@click="compare"
			>
				Compare
			</Button>
		</div>
	</div>
</template>

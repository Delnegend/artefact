<script setup lang="ts">
import { Info } from 'lucide-vue-next'
import Button from '~/components/ui/button/Button.vue'
import Input from '~/components/ui/input/Input.vue'
import {
	Popover,
	PopoverContent,
	PopoverTrigger
} from '~/components/ui/popover'
import {
	useProcessConfigStore,
	processConfigStoreOps
} from '~/composables/use-process-config-store'
import { OutputImgFormat } from '~/utils/types'

const processConfig = useProcessConfigStore()
const isDefault = processConfigStoreOps.isDefault()
</script>

<template>
	<div class="flex flex-col gap-6 text-left">
		<hr />
		<div>
			<div class="mb-2">Output format</div>
			<div class="grid w-full grid-cols-4 gap-2">
				<Button
					v-for="format in [
						OutputImgFormat.PNG,
						OutputImgFormat.WEBP,
						OutputImgFormat.TIF,
						OutputImgFormat.BMP
					]"
					:key="format"
					:variant="
						processConfig.outputFormat === format
							? 'default'
							: 'secondary'
					"
					@click="
						() => {
							processConfig.outputFormat = format
						}
					"
				>
					{{ format }}
				</Button>
			</div>
		</div>
		<div>
			<div class="mb-2 flex flex-row items-center gap-2">
				<div class="text-lg">Iterations</div>
				<Popover>
					<PopoverTrigger>
						<Info class="size-5 opacity-75" />
					</PopoverTrigger>
					<PopoverContent>
						<p class="text-sm text-balance text-primary/60">
							The number of optimization steps and are represented
							as an integer. Higher values yield better results
							but require more time. The iterations for the chroma
							components default to the luma iterations.
						</p>
					</PopoverContent>
				</Popover>
			</div>
			<Input
				v-model.number="processConfig.iterations"
				step="10"
				type="number"
				@blur="processConfigStoreOps.ensureInterationsValid"
			/>
		</div>
		<div>
			<div class="mb-2 flex flex-row items-center gap-2">
				<div class="text-lg">Weight</div>
				<Popover>
					<PopoverTrigger>
						<Info class="size-5 opacity-75" />
					</PopoverTrigger>
					<PopoverContent>
						<p class="text-sm text-balance text-primary/60">
							A floating point number for Total Generalized
							Variation weight. Higher values result in smoother
							transitions with less staircasing. A value of 1.0
							means equivalent weight to the first order weight,
							while a value of 0.0 means plain Total Variation and
							provides a speed boost. Weights for the chroma
							components always default to 0.
						</p>
					</PopoverContent>
				</Popover>
			</div>

			<Input
				v-model.number="processConfig.weight"
				type="number"
				step="0.01"
				@blur="processConfigStoreOps.ensureWeightValid"
			/>
		</div>
		<div>
			<div class="mb-2 flex flex-row items-center gap-2">
				<div class="text-lg">Pweight</div>
				<Popover>
					<PopoverTrigger>
						<Info class="size-5 opacity-75" />
					</PopoverTrigger>
					<PopoverContent>
						<p class="text-sm text-balance text-primary/60">
							A floating-point number for DCT coefficient distance
							weight. Higher values make the result more similar
							to the source JPEG. A value of 1.0 means
							approximately equivalent weight to the first-order
							weight, while a value of 0.0 means to ignore this
							and provides a speed boost. Weights for the chroma
							components default to the luma weight.
						</p>
					</PopoverContent>
				</Popover>
			</div>
			<Input
				v-model.number="processConfig.pWeight"
				type="number"
				step="0.01"
				@blur="processConfigStoreOps.ensurePWeightValid"
			/>
		</div>
		<Button
			:disabled="isDefault"
			@click="processConfigStoreOps.resetDefaultAll"
		>
			Reset to default
		</Button>
	</div>
</template>

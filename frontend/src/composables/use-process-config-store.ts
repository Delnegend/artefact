import { clamp } from '@vueuse/core'
import { useState } from 'nuxt/app'
import { computed, watchEffect, type Ref } from 'vue'
import { OutputImgFormat, type ProcessingConfig } from '~/utils/types'

export function useProcessConfigStore(): Ref<ProcessingConfig> {
	return useState<ProcessingConfig>('processing-config', () => ({
		outputFormat: OutputImgFormat.PNG,
		iterations: 50,
		weight: 0.3,
		pWeight: 0.001,
		separateComponents: false
	}))
}

export const processConfigStoreOps = {
	init(): void {
		const processConfigStore = useProcessConfigStore()
		processConfigStore.value = JSON.parse(
			localStorage.getItem('processing-config') ?? '{}'
		) as ProcessingConfig

		watchEffect(() => {
			localStorage.setItem(
				'processing-config',
				JSON.stringify(processConfigStore.value)
			)
		})
	},

	resetDefaultAll(): void {
		const processConfigStore = useProcessConfigStore()
		processConfigStore.value = {
			outputFormat: OutputImgFormat.PNG,
			iterations: 50,
			weight: 0.3,
			pWeight: 0.001,
			separateComponents: false
		}
	},

	save(): void {
		const processConfigStore = useProcessConfigStore()
		localStorage.setItem(
			'processing-config',
			JSON.stringify(processConfigStore.value)
		)
	},

	isDefault(): Ref<boolean> {
		const processConfigStore = useProcessConfigStore()
		return computed(() => {
			return (
				processConfigStore.value.outputFormat === OutputImgFormat.PNG &&
				processConfigStore.value.iterations === 50 &&
				processConfigStore.value.weight === 0.3 &&
				processConfigStore.value.pWeight === 0.001 &&
				!processConfigStore.value.separateComponents
			)
		})
	},

	ensureInterationsValid(): void {
		const processConfigStore = useProcessConfigStore()
		processConfigStore.value.iterations = clamp(
			processConfigStore.value.iterations,
			1,
			1000
		)
	},

	ensureWeightValid(): void {
		const processConfigStore = useProcessConfigStore()
		processConfigStore.value.weight = clamp(
			processConfigStore.value.weight,
			0,
			1
		)
	},

	ensurePWeightValid(): void {
		const processConfigStore = useProcessConfigStore()
		processConfigStore.value.pWeight = clamp(
			processConfigStore.value.pWeight,
			0,
			1
		)
	}
}

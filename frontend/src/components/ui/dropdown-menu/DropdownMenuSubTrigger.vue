<script setup lang="ts">
import { ChevronRight } from 'lucide-vue-next'
import {
	DropdownMenuSubTrigger,
	type DropdownMenuSubTriggerProps,
	useForwardProps
} from 'radix-vue'
import { computed, type HTMLAttributes } from 'vue'
import { cn } from '~/utils/cn'

const props = defineProps<
	DropdownMenuSubTriggerProps & { class?: HTMLAttributes['class'] }
>()

const delegatedProps = computed(() => {
	const { class: _, ...delegated } = props

	return delegated
})

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
	<DropdownMenuSubTrigger
		v-bind="forwardedProps"
		:class="
			cn(
				'flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-hidden focus:bg-accent data-[state=open]:bg-accent',
				props.class
			)
		"
	>
		<slot />
		<ChevronRight class="ml-auto size-4" />
	</DropdownMenuSubTrigger>
</template>

<script setup lang="ts">
import {
	DropdownMenuItem,
	type DropdownMenuItemProps,
	useForwardProps
} from 'radix-vue'
import { computed, type HTMLAttributes } from 'vue'
import { cn } from '~/utils/cn'

const props = defineProps<
	DropdownMenuItemProps & { class?: HTMLAttributes['class']; inset?: boolean }
>()

const delegatedProps = computed(() => {
	const { class: _, ...delegated } = props

	return delegated
})

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
	<DropdownMenuItem
		v-bind="forwardedProps"
		:class="
			cn(
				'relative flex cursor-default select-none items-center rounded-sm gap-2 px-2 py-1.5 text-sm outline-hidden transition-colors focus:bg-accent focus:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50  [&>svg]:size-4 [&>svg]:shrink-0',
				inset && 'pl-8',
				props.class
			)
		"
	>
		<slot />
	</DropdownMenuItem>
</template>

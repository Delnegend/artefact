<script setup lang="ts">
import {
	SplitterGroup,
	type SplitterGroupEmits,
	type SplitterGroupProps,
	useForwardPropsEmits
} from 'radix-vue'
import { computed, type HTMLAttributes } from 'vue'
import { cn } from '~/utils/cn'

const props = defineProps<
	SplitterGroupProps & { class?: HTMLAttributes['class'] }
>()
const emits = defineEmits<SplitterGroupEmits>()

const delegatedProps = computed(() => {
	// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	const { class: _, ...delegated } = props
	return delegated
})

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
	<SplitterGroup
		v-bind="forwarded"
		:class="
			cn(
				'flex h-full w-full data-[panel-group-direction=vertical]:flex-col',
				props.class
			)
		"
	>
		<slot />
	</SplitterGroup>
</template>

<script setup lang="ts">
import { DialogTitle, type DialogTitleProps } from 'radix-vue'
import { computed, type HTMLAttributes } from 'vue'
import { cn } from '~/utils/cn'

const props = defineProps<
	DialogTitleProps & { class?: HTMLAttributes['class'] }
>()

const delegatedProps = computed(() => {
	// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	const { class: _, ...delegated } = props

	return delegated
})
</script>

<template>
	<DialogTitle
		:class="cn('text-lg font-semibold text-foreground', props.class)"
		v-bind="delegatedProps"
	>
		<slot />
	</DialogTitle>
</template>

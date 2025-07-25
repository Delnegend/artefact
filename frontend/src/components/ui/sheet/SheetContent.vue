<script setup lang="ts">
import { sheetBaseClass, sheetSide } from '.'
import { X } from 'lucide-vue-next'
import {
	DialogClose,
	DialogContent,
	type DialogContentEmits,
	type DialogContentProps,
	DialogOverlay,
	DialogPortal,
	useForwardPropsEmits
} from 'radix-vue'
import { computed, type HTMLAttributes } from 'vue'
import { cn } from '~/utils/cn'

defineOptions({
	inheritAttrs: false
})

const props = defineProps<
	DialogContentProps & {
		class?: HTMLAttributes['class']
		side?: keyof typeof sheetSide
		description: string
	}
>()

const emits = defineEmits<DialogContentEmits>()

const delegatedProps = computed(() => {
	// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	const { class: _, side: _side, ...delegated } = props

	return delegated
})

const forwarded = useForwardPropsEmits(delegatedProps, emits)
</script>

<template>
	<DialogPortal>
		<DialogOverlay
			class="fixed inset-0 z-50 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
		/>
		<DialogContent
			:class="
				cn(sheetBaseClass, sheetSide[props.side ?? 'left'], props.class)
			"
			v-bind="{ ...forwarded, ...$attrs }"
			:aria-describedby="props.description"
		>
			<slot />

			<DialogClose
				class="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-secondary"
			>
				<X class="size-4 text-muted-foreground" />
			</DialogClose>
		</DialogContent>
	</DialogPortal>
</template>

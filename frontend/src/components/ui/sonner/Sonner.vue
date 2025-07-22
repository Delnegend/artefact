<script lang="ts" setup>
import { Toaster as Sonner, type ToasterProps } from 'vue-sonner'

const props = defineProps<ToasterProps>()
</script>

<template>
	<Sonner
		class="toaster group fixed flex flex-col gap-3 bottom-4 right-4 z-50 w-sm max-w-[92dvw] [&>li]:rounded-lg [&>li]:border [&>li]:border-border [&>li]:bg-popover [&>li]:p-4 [&>li]:text-popover-foreground [&>li]:shadow-lg [&>li]:transition-all dark:[&>li]:bg-popover dark:[&>li]:text-popover-foreground [&>li]:flex [&>li]:flex-row [&>li]:gap-3 [&>li]:items-center"
		v-bind="props"
		:style="{
			'--normal-bg': 'var(--popover)',
			'--normal-text': 'var(--popover-foreground)',
			'--normal-border': 'var(--border)'
		}"
	/>
</template>

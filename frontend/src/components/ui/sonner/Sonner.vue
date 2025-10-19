<script lang="ts" setup>
import { Toaster as Sonner, type ToasterProps } from 'vue-sonner'

const props = defineProps<ToasterProps>()
</script>

<template>
	<Sonner
		class="toaster group fixed right-4 bottom-4 z-50 flex w-sm max-w-[92dvw] flex-col gap-3 [&>li]:flex [&>li]:flex-row [&>li]:items-center [&>li]:gap-3 [&>li]:rounded-lg [&>li]:border [&>li]:border-border [&>li]:bg-popover [&>li]:p-4 [&>li]:text-popover-foreground [&>li]:shadow-lg [&>li]:transition-all dark:[&>li]:bg-popover dark:[&>li]:text-popover-foreground"
		v-bind="props"
		:style="{
			'--normal-bg': 'var(--popover)',
			'--normal-text': 'var(--popover-foreground)',
			'--normal-border': 'var(--border)'
		}"
	/>
</template>

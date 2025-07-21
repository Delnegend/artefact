<script setup lang="ts">
import { toast } from 'vue-sonner'
import { cn } from '~/utils/cn'

const props = defineProps<{ class?: string }>()

function WIP(): void {
	toast.info('This feature is not yet implemented')
}
</script>

<template>
	<div :class="cn('gap-2 p-4 left-0 grid grid-cols-2', props.class)">
		<Button variant="outline" @click="WIP"> Process all </Button>
		<Button variant="destructive" @click="WIP"> Remove all </Button>
		<Button class="col-span-2" @click="WIP"> Download processed </Button>
	</div>
</template>

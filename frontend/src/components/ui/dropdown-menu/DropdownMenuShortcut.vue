<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '~/utils/cn'

const props = defineProps<{
	class?: HTMLAttributes['class']
}>()
</script>

<template>
	<span
		:class="cn('ml-auto text-xs tracking-widest opacity-60', props.class)"
	>
		<slot />
	</span>
</template>

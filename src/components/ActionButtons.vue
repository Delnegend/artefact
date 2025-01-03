<script setup lang="ts">
import { cn } from "@/lib/utils";

const props = defineProps<{
	class?: string;
}>();
</script>

<template>
	<div :class="cn('gap-2 p-4 left-0 grid grid-cols-2', props.class)">
		<Button variant="outline">
			Process all
		</Button>
		<Button variant="destructive">
			Remove all
		</Button>
		<Button class="col-span-2">
			Download processed
		</Button>
	</div>
</template>

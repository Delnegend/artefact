<script setup lang="ts">
import { DialogDescription, type DialogDescriptionProps } from "radix-vue";
import { computed, type HTMLAttributes } from "vue";

import { cn } from "~/utils/cn";

const props = defineProps<DialogDescriptionProps & { class?: HTMLAttributes["class"] }>();

const delegatedProps = computed(() => {
	// eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
	const { class: _, ...delegated } = props;

	return delegated;
});
</script>

<template>
	<DialogDescription
		:class="cn('text-sm text-muted-foreground', props.class)"
		v-bind="delegatedProps">
		<slot />
	</DialogDescription>
</template>

<script setup lang="ts">
import {
	DropdownMenuSeparator,
	type DropdownMenuSeparatorProps,
} from "radix-vue";
import { computed, type HTMLAttributes } from "vue";

import { cn } from "@/lib/utils";

const props = defineProps<DropdownMenuSeparatorProps & {
	class?: HTMLAttributes["class"];
}>();

const delegatedProps = computed(() => {
	// eslint-disable-next-line no-unused-vars, @typescript-eslint/no-unsafe-assignment
	const { class: _, ...delegated } = props;

	return delegated;
});
</script>

<template>
	<DropdownMenuSeparator v-bind="delegatedProps" :class="cn('-mx-1 my-1 h-px bg-muted', props.class)" />
</template>

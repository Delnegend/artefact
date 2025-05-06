<script setup lang="ts">
import { cn } from "~/utils/cn";

const badgeBaseClass = "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2";

const badgeVariants = {
	default: "border-transparent bg-primary text-primary-foreground hover:bg-primary/80",
	secondary: "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/80",
	destructive: "border-transparent bg-destructive text-destructive-foreground hover:bg-destructive/80",
	outline: "text-foreground",
};

const props = defineProps<{
	variant?: keyof typeof badgeVariants;
	class?: string;
}>();
</script>

<template>
	<div :class="cn(badgeBaseClass, badgeVariants[props.variant ?? 'default'], props.class ?? '')">
		<slot />
	</div>
</template>

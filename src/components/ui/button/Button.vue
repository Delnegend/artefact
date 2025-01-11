<script setup lang="ts">
import { Primitive, type PrimitiveProps } from "radix-vue";
import type { HTMLAttributes } from "vue";

import { cn } from "~/utils";

import { buttonBaseClassTw, buttonSizesTw, buttonVariantsTw } from "./index";

interface Props extends PrimitiveProps {
	variant?: keyof typeof buttonVariantsTw;
	size?: keyof typeof buttonSizesTw;
	class?: HTMLAttributes["class"];
}

const props = withDefaults(defineProps<Props>(), {
	as: "button",
	variant: "default",
	size: "default",
	class: "",
});
</script>

<template>
	<Primitive
		:as="as"
		:as-child="asChild"
		:class="cn(buttonBaseClassTw, buttonVariantsTw[props.variant], buttonSizesTw[props.size], props.class)"
	>
		<slot />
	</Primitive>
</template>

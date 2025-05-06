import { ref } from "vue";

import type { ResizablePanel } from "~/components/ui/resizable";

export const displayMode = ref("horizontal" as "horizontal" | "vertical");

export const colorScheme = ref<"light" | "dark">("light");

export const imageInputPanelRef = ref<InstanceType<typeof ResizablePanel>>();


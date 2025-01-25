import { ref } from "vue";

import type { ResizablePanel } from "~/components/ui/resizable";

export const displayMode = ref("horizontal" as "horizontal" | "vertical");

export const colorScheme = ref<"light" | "dark">("light");

export const imageInputPanelRef = ref<InstanceType<typeof ResizablePanel>>();

export * from "./use-image-compare-store";
export * from "./use-process-config-store";
export * from "./use-simple-artefact-worker";


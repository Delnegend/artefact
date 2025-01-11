import { type Ref, ref } from "vue";

import type { ResizablePanel } from "~/components/ui/resizable";
import type { ImageItemForDisplay } from "~/utils";

export const displayMode = ref("horizontal" as "horizontal" | "vertical");

type JpegFileHash = string;
export const imageDisplayList: Ref<Map<JpegFileHash, ImageItemForDisplay>> = ref(new Map());

export const colorScheme = ref<"light" | "dark">("light");

export const imageInputPanelRef = ref<InstanceType<typeof ResizablePanel>>();

export * from "./use-artefact-worker";
export * from "./use-image-compare-store";
export * from "./use-process-config-store";


import { ref, type Ref } from 'vue'

interface Internal {
	jpegBlobUrl: string | undefined
	outputImgBlobUrl: string | undefined
	compareMode: 'side-by-side' | 'overlay'
}

const store = ref<Internal>({
	jpegBlobUrl: undefined,
	outputImgBlobUrl: undefined,
	compareMode: 'overlay'
})

export function useImageCompareStore(): Ref<Internal> {
	return store
}

import { hashArrayBuffer } from '../utils/hash-array-buffer'
import { useState } from 'nuxt/app'
import type { Ref } from 'vue'
import { deleteFileInDb, getAllFilesInDb, putFilesInDb } from '~/utils/db'
import type { ImageItemForDisplay } from '~/utils/types'

export function useImageListStore(): Ref<Record<string, ImageItemForDisplay>> {
	return useState<Record<string, ImageItemForDisplay>>(
		'image-list',
		() => ({})
	)
}

export const imageListStoreOps = {
	async init(): Promise<void> {
		const imageListStore = useImageListStore()
		imageListStore.value = (await getAllFilesInDb()).reduce<
			Record<string, ImageItemForDisplay>
		>((acc, image) => {
			acc[image.jpegFileHash] = {
				name: image.jpegFileName,
				dateAdded: image.dateAdded,
				size: image.jpegFileSize,
				jpegBlobUrl: URL.createObjectURL(
					new Blob([image.jpegArrayBuffer], { type: 'image/jpeg' })
				),
				outputImgBlobUrl: image.outputImgArrayBuffer
					? URL.createObjectURL(
							new Blob([image.outputImgArrayBuffer], {
								type: `image/${image.outputImgFormat}`
							})
						)
					: undefined,
				outputImgFormat: image.outputImgFormat,
				width: image.width,
				height: image.height
			}
			return acc
		}, {})
	},

	async addFileList(fileList: FileList | null): Promise<void> {
		if (!fileList) {
			return
		}

		const imageListStore = useImageListStore()

		const fileOps = await Promise.all(
			Array.from(fileList).map(async (file) => {
				const jpegArrayBuffer = await file.arrayBuffer()
				const hash = await hashArrayBuffer(jpegArrayBuffer)

				const img = new Image()
				img.src = URL.createObjectURL(
					new Blob([jpegArrayBuffer], { type: 'image/jpeg' })
				)
				await new Promise((resolve) => {
					img.onload = resolve
				})

				return {
					file,
					jpegArrayBuffer,
					hash,
					width: img.width,
					height: img.height
				}
			})
		)

		imageListStore.value = {
			...imageListStore.value,
			...Object.fromEntries(
				fileOps.map(
					({ file, jpegArrayBuffer, hash, width, height }) => {
						return [
							hash,
							{
								name: file.name,
								dateAdded: new Date(),
								size: jpegArrayBuffer.byteLength,
								jpegBlobUrl: URL.createObjectURL(
									new Blob([jpegArrayBuffer], {
										type: 'image/jpeg'
									})
								),
								width,
								height
							}
						]
					}
				)
			)
		}

		void putFilesInDb(
			fileOps.map(({ file, jpegArrayBuffer, hash, width, height }) => {
				return {
					jpegFileHash: hash,
					jpegFileName: file.name,
					dateAdded: new Date(),
					jpegFileSize: jpegArrayBuffer.byteLength,
					jpegArrayBuffer,
					width,
					height
				}
			})
		)
	},

	remove(jpegFileHash: string): void {
		const imageListStore = useImageListStore()

		imageListStore.value = Object.fromEntries(
			Object.entries(imageListStore.value).filter(
				([key]) => key !== jpegFileHash
			)
		)
		void deleteFileInDb(jpegFileHash)
	}
} as const

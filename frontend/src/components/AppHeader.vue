<script setup lang="ts">
import { Columns2, Moon, Rows2, Settings, Sun } from 'lucide-vue-next'
import { onBeforeMount, watch } from 'vue'
import AppDrawer from '~/components/AppDrawer.vue'
import Button from '~/components/ui/button/Button.vue'
import {
	Sheet,
	SheetContent,
	SheetHeader,
	SheetTitle,
	SheetTrigger
} from '~/components/ui/sheet'
import { colorScheme, displayMode } from '~/composables'

onBeforeMount(() => {
	const osColorScheme = window.matchMedia('(prefers-color-scheme: dark)')
		.matches
		? 'dark'
		: 'light'
	const configuredColorScheme = window.localStorage.getItem(
		'color-scheme'
	) as 'light' | 'dark' | null
	colorScheme.value = configuredColorScheme ?? osColorScheme
})

watch(colorScheme, (newColorScheme) => {
	window.localStorage.setItem('color-scheme', newColorScheme)
	if (!document.documentElement.classList.contains('dark')) {
		document.documentElement.classList.add('dark')
		return
	}
	document.documentElement.classList.remove('dark')
})

function handleDisplayModeChange(): void {
	switch (displayMode.value) {
		case 'horizontal': {
			displayMode.value = 'vertical'
			break
		}
		case 'vertical': {
			displayMode.value = 'horizontal'
			break
		}
		default:
			displayMode.value = 'horizontal'
	}
}
</script>

<template>
	<header
		class="grid h-16 w-full grid-cols-[1fr_auto_1fr] items-center border-b px-4"
	>
		<div
			class="flex gap-3 [&>button]:aspect-square [&>button]:size-10 [&>button]:p-0"
		>
			<Sheet>
				<SheetTrigger as-child>
					<Button variant="outline" class="aspect-square size-10 p-0">
						<Settings />
					</Button>
				</SheetTrigger>
				<SheetContent
					side="left"
					class="items-start"
					description="Configuration sheet"
				>
					<SheetHeader>
						<SheetTitle>Configurations</SheetTitle>
						<AppDrawer />
					</SheetHeader>
				</SheetContent>
			</Sheet>

			<Button variant="outline" @click="handleDisplayModeChange">
				<Columns2 v-if="displayMode === 'vertical'" />
				<Rows2 v-else-if="displayMode === 'horizontal'" />
			</Button>
		</div>

		<div class="relative select-none">
			<div class="text-4xl font-black italic drop-shadow-xl">
				artefact
			</div>
			<div
				class="absolute -right-6 -bottom-6 z-10 rounded-full bg-primary px-2 py-1 text-sm font-black whitespace-nowrap text-red-500 italic drop-shadow-md"
			>
				beta
			</div>
		</div>

		<div class="flex flex-row items-center gap-4 justify-self-end">
			<Button
				variant="outline"
				class="size-10"
				@click="
					colorScheme = colorScheme === 'light' ? 'dark' : 'light'
				"
			>
				<Moon v-if="colorScheme === 'light'" />
				<Sun v-else />
			</Button>

			<a
				href="https://github.com/Delnegend/artefact"
				target="_blank"
				rel="noopener noreferrer"
			>
				<div
					class="aspect-square size-6 transition-transform hover:scale-110 dark:invert"
				>
					<svg
						role="img"
						viewBox="0 0 24 24"
						xmlns="http://www.w3.org/2000/svg"
					>
						<title>GitHub</title>
						<path
							d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"
						/>
					</svg>
				</div>
			</a>
		</div>
	</header>
</template>

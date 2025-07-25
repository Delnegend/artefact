export { default as Button } from './Button.vue'

export const buttonBaseClassTw =
	'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0 select-none'

export const buttonVariantsTw = {
	default: 'bg-primary text-primary-foreground shadow-sm hover:bg-primary/90',
	destructive:
		'bg-destructive text-destructive-foreground shadow-xs hover:bg-destructive/90',
	outline:
		'border border-input bg-background shadow-xs hover:bg-accent hover:text-accent-foreground',
	secondary:
		'bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80',
	ghost: 'hover:bg-accent hover:text-accent-foreground',
	link: 'text-primary underline-offset-4 hover:underline'
} as const

export const buttonSizesTw = {
	default: 'h-9 px-4 py-2',
	sm: 'h-8 rounded-md px-3 text-xs',
	lg: 'h-10 rounded-md px-8',
	icon: 'h-9 w-9'
} as const

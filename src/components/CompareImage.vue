<script setup lang="ts">
const app = ref<HTMLElement | null>(null);

const props = defineProps<{
	imageAUrl: string;
	imageBUrl: string;
}>();

const isReady = ref(false);

onMounted(() => {
	if (!app.value) { return; }

	isReady.value = true;

	// @ts-expect-error - NVIDIA ICAT thing
	app.value.isEmbed = true;
	// @ts-expect-error - NVIDIA ICAT thing
	app.value.loadAppData({
		activeSet: "main-set",
		uiVisibility: {
			antiAliasSelect: true,
			fullscreenButton: true,
			gridSelect: true,
			helpButton: true,
			layersButton: true,
			layoutTabs: true,
			loopButton: true,
			mediaInfo: true,
			pixelDataButton: false,
			progressBar: true,
			setSelect: true,
			speedSelect: true,
			startEndTime: false,
			timeDisplay: true,
			titleBar: false,
			videoControls: true,
			zoomSelect: true,
			contentSidebar: false,
			timeline: false,
			controlBar: true,
			addFilterBtn: false,
			addLogoBtn: false,
			inOutSlider: false,
			timelineInOutSlider: false,
			appSettingsBtn: false,
		},
		sets: [{
			id: "main-set",
			// "created": 1735542741590,
			label: "Set 1",
			antiAlias: false,
			// "endTime": 1e+29,
			// "grid": 0,
			layout: "tile",
			overlayOrder: [],
			// "looping": true,
			// "boomerang": false,
			mediaOrder: ["image-b", "image-a"],
			// "x": -85,
			// "y": -24,
			// "z": 100,
			// "speed": 1,
			// "splitposition": 0.5,
			// "startTime": 0,
			// "time": 0,
			// "viewport": {
			// 	"top": 44,
			// 	"left": 0,
			// 	"paneWidth": 0,
			// 	"paneHeight": 0,
			// 	"height": 0,
			// 	"width": 0,
			// 	"minZoom": 0,
			// 	"fitScale": 1,
			// 	"fitWidth": 2246,
			// 	"fitHeight": 794,
			// 	"fitPaneWidth": 1123,
			// 	"fitPaneHeight": 794,
			// 	"contentWidth": 1123,
			// 	"contentHeight": 794
			// },
			gamma: 1,
			exposure: 1,
			media: [{
				id: "image-a",
				// "created": 1735542741617,
				codec: "PNG",
				bitrate: 0,
				// "width": 1123,
				// "height": 794,
				type: "image",
				// "fps": 0,
				// "duration": 0,
				// "size": 1173753,
				// "modified": 1735542738653,
				// "hasAudio": false,
				// "timestamps": [],
				path: "after.png",
				name: "after.png",
				base: "after",
				originalBase: "after",
				extension: "png",
				format: "PNG",
				// "x": 0,
				// "y": 0,
				// "z": 100,
				// "startTime": 0,
				// "thumbnails": [],
				// "labelVisibility": {
				// 	"codec": false,
				// 	"bitrate": false,
				// 	"dimensions": false,
				// 	"format": false,
				// 	"fps": false,
				// 	"duration": false,
				// 	"startTime": false
				// },
				muted: true,
				visible: true,
				// "shaderId": "",
				// "shaderInputs": [],
				// "shaderSettings": {},
				// "shaderSource": "",
				// "showShader": true,
				src: "after.png",
				// "mediaOverlay": "",
				// "frameImages": [],
				// "showFrameImages": true,
				// "infoTexts": [],
				// "showInfoText": false
			},
			{
				id: "image-b",
				// "created": 1735542741621,
				codec: "JPG",
				// "bitrate": 0,
				// "width": 1123,
				// "height": 794,
				type: "image",
				// "fps": 0,
				// "duration": 0,
				// "size": 540458,
				// "modified": 1735541986293,
				// "hasAudio": false,
				// "timestamps": [],
				path: "before.jpg",
				name: "before.jpg",
				base: "before",
				originalBase: "before",
				extension: "jpg",
				format: "JPG",
				// "x": 0,
				// "y": 0,
				// "z": 100,
				// "startTime": 0,
				// "thumbnails": [],
				// "labelVisibility": {
				// 	"codec": false,
				// 	"bitrate": false,
				// 	"dimensions": false,
				// 	"format": false,
				// 	"fps": false,
				// 	"duration": false,
				// 	"startTime": false
				// },
				// "muted": true,
				// "visible": true,
				// "shaderId": "",
				// "shaderInputs": [],
				// "shaderSettings": {},
				// "shaderSource": "",
				showShader: true,
				src: "before.jpg",
				// "mediaOverlay": "",
				// "frameImages": [],
				showFrameImages: true,
				// "infoTexts": [],
				showInfoText: false,
			}],
			// "overlays": []
		}],
	} as const);
});

watchEffect(() => {
	if (!isReady.value) {}
});

</script>

<template>
	<icat-app id="icat-196742f600000023844fac9aebd5a15d9441" ref="app" />
</template>

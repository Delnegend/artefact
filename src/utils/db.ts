import { openDB } from "idb";

export const db = await openDB("artefact", 20250109, {
	upgrade(db, oldVersion, newVersion) {
		const alreadyExists = db.objectStoreNames.contains("files");

		if (newVersion !== null && alreadyExists && oldVersion !== newVersion) {
			db.deleteObjectStore("files");
		}

		if (!db.objectStoreNames.contains("files")) {
			db.createObjectStore("files", {
				keyPath: "jpegFileHash",
				autoIncrement: false,
			});
		}
	},
});

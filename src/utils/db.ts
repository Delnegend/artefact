import type { ImageItemForDB } from "./types";

const DB_NAME = "artefact";
const DB_VERSION = 20250109;
const STORE_NAME = "files";

let dbPromise: Promise<IDBDatabase> | null = null;

function getDb(): Promise<IDBDatabase> {
	if (dbPromise) {
		return dbPromise;
	}

	dbPromise = new Promise((resolve, reject) => {
		// Determine the correct IndexedDB context (window or worker)
		// eslint-disable-next-line no-nested-ternary, @typescript-eslint/no-unnecessary-condition, @typescript-eslint/strict-boolean-expressions
		const idb = typeof window !== "undefined" && window.indexedDB
			? window.indexedDB
			// eslint-disable-next-line @typescript-eslint/no-unnecessary-condition, @typescript-eslint/strict-boolean-expressions
			: typeof self !== "undefined" && self.indexedDB
				? self.indexedDB
				: null;
		// eslint-enable

		if (!idb) {
			const errorMsg = "IndexedDB is not supported in this environment.";
			console.error(errorMsg);
			reject(new Error(errorMsg));
			dbPromise = null; // Reset promise
			return; // Exit promise executor
		}

		const request = idb.open(DB_NAME, DB_VERSION);

		request.onupgradeneeded = (event): void => {
			const db = request.result;
			const target = event.target as IDBOpenDBRequest | null;
			const transaction = target?.transaction;

			if (!transaction) {
				console.error("Upgrade transaction is null");
				return;
			}

			const oldVersion = event.oldVersion;
			const newVersion = event.newVersion;

			const storeExists = db.objectStoreNames.contains(STORE_NAME);

			if (newVersion !== null && storeExists && oldVersion !== newVersion) {
				// Re-create store only if version changes and store exists
				// Note: Simple deletion might not be ideal for data migration in real apps.
				db.deleteObjectStore(STORE_NAME);
			}

			if (!db.objectStoreNames.contains(STORE_NAME)) {
				db.createObjectStore(STORE_NAME, {
					keyPath: "jpegFileHash",
					autoIncrement: false,
				});
			}
		};

		request.onsuccess = (): void => {
			resolve(request.result);
		};

		request.onerror = (): void => {
			const err = `Database error: ${request.error?.message}`;
			console.error(err);
			reject(new Error(err));
			dbPromise = null; // Reset promise on error
		};

		request.onblocked = (): void => {
			console.warn("Database open blocked, please close other tabs/windows using this database.");
			// Optionally reject or keep waiting
			// reject(new Error("Database open blocked"));
			// dbPromise = null; // Reset promise if rejecting
		};
	});

	return dbPromise;
}

export async function getAllFilesInDb(): Promise<ImageItemForDB[]> {
	const db = await getDb();
	return new Promise((resolve, reject) => {
		const transaction = db.transaction(STORE_NAME, "readonly");
		const store = transaction.objectStore(STORE_NAME);
		const request = store.getAll();

		request.onsuccess = (): void => {
			resolve(request.result as ImageItemForDB[]);
		};

		request.onerror = (): void => {
			const err = "Error getting all files.";
			console.error(err);
			reject(new Error(err));
		};
	});
}

export async function getFileInDb(jpegFileHash: string): Promise<ImageItemForDB | undefined> {
	const db = await getDb();
	return new Promise((resolve, reject) => {
		const transaction = db.transaction(STORE_NAME, "readonly");
		const store = transaction.objectStore(STORE_NAME);
		const request = store.get(jpegFileHash);

		request.onsuccess = (): void => {
			resolve(request.result as ImageItemForDB | undefined);
		};

		request.onerror = (): void => {
			const err = `Error getting file ${jpegFileHash}: ${request.error?.message}`;
			console.error(err);
			reject(new Error(err));
		};
	});
}

export async function putFilesInDb(files: ImageItemForDB[]): Promise<void> {
	const db = await getDb();
	return new Promise((resolve, reject) => {
		const transaction = db.transaction(STORE_NAME, "readwrite");
		const store = transaction.objectStore(STORE_NAME);
		let completed = 0;
		let hasError = false;

		if (files.length === 0) {
			resolve();
			return;
		}

		for (const file of files) {
			const request = store.put(file);
			// eslint-disable-next-line @typescript-eslint/no-loop-func
			request.onsuccess = (): void => {
				completed++;
				if (!hasError && completed === files.length) {
					// Only resolve when all puts are successful *and* transaction completes
				}
			};
			// eslint-disable-next-line @typescript-eslint/no-loop-func
			request.onerror = (): void => {
				if (!hasError) { // Report first error only
					hasError = true;
					console.error("Error putting file:", request.error);
					// Don't reject here, let the transaction error handler do it
					transaction.abort(); // Abort the transaction on any put error
				}
			};
		}

		transaction.oncomplete = (): void => {
			if (!hasError) {
				resolve();
			}
			// If hasError is true, the onerror handler below will reject
		};

		transaction.onerror = (): void => {
			const err = `Transaction error putting files: ${transaction.error?.message}`;
			console.error(err);
			reject(new Error(err));
		};

		transaction.onabort = (): void => {
			const err = "Transaction aborted putting files.";
			console.warn(err);
			reject(new Error(err));
		};
	});
}

export async function deleteFileInDb(jpegFileHash: string): Promise<void> {
	const db = await getDb();
	return new Promise((resolve, reject) => {
		const transaction = db.transaction(STORE_NAME, "readwrite");
		const store = transaction.objectStore(STORE_NAME);
		const request = store.delete(jpegFileHash);

		request.onsuccess = (): void => {
			// Resolve on transaction complete, not just request success
		};

		request.onerror = (): void => {
			const err = `Error deleting file ${jpegFileHash}: ${request.error?.message}`;
			console.error(err);
			reject(new Error(err));
		};

		transaction.oncomplete = (): void => {
			resolve();
		};

		transaction.onerror = (): void => {
			const err = `Transaction error deleting file: ${transaction.error?.message}`;
			console.error(err);
			reject(new Error(err));
		};
	});
}

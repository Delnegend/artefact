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
		const idb = typeof window !== 'undefined' && window.indexedDB
			? window.indexedDB
			: typeof self !== 'undefined' && self.indexedDB
				? self.indexedDB
				: null;

		if (!idb) {
			const errorMsg = "IndexedDB is not supported in this environment.";
			console.error(errorMsg);
			reject(new Error(errorMsg));
			dbPromise = null; // Reset promise
			return; // Exit promise executor
		}

		const request = idb.open(DB_NAME, DB_VERSION);

		request.onupgradeneeded = (event) => {
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

		request.onsuccess = () => {
			resolve(request.result);
		};

		request.onerror = () => {
			console.error("Database error:", request.error);
			reject(request.error);
			dbPromise = null; // Reset promise on error
		};

		request.onblocked = () => {
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

		request.onsuccess = () => {
			resolve(request.result as ImageItemForDB[]);
		};

		request.onerror = () => {
			console.error("Error getting all files:", request.error);
			reject(request.error);
		};
	});
}

export async function getFileInDb(jpegFileHash: string): Promise<ImageItemForDB | undefined> {
	const db = await getDb();
	return new Promise((resolve, reject) => {
		const transaction = db.transaction(STORE_NAME, "readonly");
		const store = transaction.objectStore(STORE_NAME);
		const request = store.get(jpegFileHash);

		request.onsuccess = () => {
			resolve(request.result as ImageItemForDB | undefined);
		};

		request.onerror = () => {
			console.error(`Error getting file ${jpegFileHash}:`, request.error);
			reject(request.error);
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

		files.forEach((file) => {
			const request = store.put(file);
			request.onsuccess = () => {
				completed++;
				if (!hasError && completed === files.length) {
					// Only resolve when all puts are successful *and* transaction completes
				}
			};
			request.onerror = () => {
				if (!hasError) { // Report first error only
					hasError = true;
					console.error("Error putting file:", request.error);
					// Don't reject here, let the transaction error handler do it
					transaction.abort(); // Abort the transaction on any put error
				}
			};
		});

		transaction.oncomplete = () => {
			if (!hasError) {
				resolve();
			}
			// If hasError is true, the onerror handler below will reject
		};

		transaction.onerror = () => {
			console.error("Transaction error putting files:", transaction.error);
			reject(transaction.error);
		};

		transaction.onabort = () => {
			console.warn("Transaction aborted putting files:", transaction.error);
			reject(transaction.error || new Error("Transaction aborted"));
		}
	});
}


export async function deleteFileInDb(jpegFileHash: string): Promise<void> {
	const db = await getDb();
	return new Promise((resolve, reject) => {
		const transaction = db.transaction(STORE_NAME, "readwrite");
		const store = transaction.objectStore(STORE_NAME);
		const request = store.delete(jpegFileHash);

		request.onsuccess = () => {
			// Resolve on transaction complete, not just request success
		};

		request.onerror = () => {
			console.error(`Error deleting file ${jpegFileHash}:`, request.error);
			reject(request.error); // Reject immediately on request error
		};

		transaction.oncomplete = () => {
			resolve();
		};

		transaction.onerror = () => {
			console.error("Transaction error deleting file:", transaction.error);
			reject(transaction.error);
		};
	});
}
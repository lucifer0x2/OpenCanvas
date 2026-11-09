/** Best effort disk storage. All transactions and opening have a short hard deadline. */
export interface SceneRecord {
  key: string;
  projectId: string;
  revision: number;
  payload: string;
  digest: string;
  bytes: number;
  used: number;
}
export const SCENE_CACHE_ENTRIES = 64;
export const SCENE_CACHE_BYTES = 32 * 1024 * 1024;
export const SCENE_CACHE_ENTRY_BYTES = 8 * 1024 * 1024;
const DEADLINE = 350;
let opening: Promise<IDBDatabase | undefined> | undefined;

function database(): Promise<IDBDatabase | undefined> {
  return (opening ??= new Promise((resolve) => {
    let done = false;
    const finish = (db?: IDBDatabase) => {
      if (done) {
        db?.close();
        return;
      }
      done = true;
      clearTimeout(timer);
      resolve(db);
    };
    const timer = setTimeout(() => finish(), DEADLINE);
    try {
      const request = indexedDB.open("opencanvas.browser-scenes", 1);
      request.onupgradeneeded = () => {
        const store = request.result.createObjectStore("scenes", {
          keyPath: "key",
        });
        store.createIndex("metadata", [
          "projectId",
          "revision",
          "used",
          "bytes",
        ]);
      };
      request.onsuccess = () => {
        const db = request.result;
        db.onversionchange = () => {
          db.close();
          opening = undefined;
        };
        finish(db);
      };
      request.onerror = request.onblocked = () => finish();
    } catch {
      finish();
    }
  }));
}

async function transaction<T>(
  mode: IDBTransactionMode,
  signal: AbortSignal,
  run: (store: IDBObjectStore, result: (value: T) => void) => void,
): Promise<T | undefined> {
  if (signal.aborted) return;
  const db = await database();
  if (!db || signal.aborted) return;
  return new Promise((resolve) => {
    let tx: IDBTransaction;
    let result: T | undefined;
    const abort = () => {
      try {
        tx?.abort();
      } catch {
        /* Already settled. */
      }
    };
    const timer = setTimeout(() => {
      abort();
      finish();
    }, DEADLINE);
    const finish = (value?: T) => {
      clearTimeout(timer);
      signal.removeEventListener("abort", abort);
      resolve(value);
    };
    try {
      tx = db.transaction("scenes", mode);
      tx.oncomplete = () => finish(result);
      tx.onerror = tx.onabort = () => finish();
      signal.addEventListener("abort", abort, { once: true });
      run(tx.objectStore("scenes"), (value) => {
        result = value;
      });
    } catch {
      finish();
    }
  });
}

export const sceneStore = {
  get(key: string, signal: AbortSignal) {
    return transaction<SceneRecord>("readonly", signal, (store, result) => {
      const request = store.get(key);
      request.onsuccess = () => result(request.result);
    });
  },
  remove(key: string, signal: AbortSignal) {
    return transaction("readwrite", signal, (store) => {
      store.delete(key);
    });
  },
  put(record: SceneRecord, signal: AbortSignal) {
    if (record.bytes > SCENE_CACHE_ENTRY_BYTES) return Promise.resolve();
    return transaction("readwrite", signal, (store) => {
      // One bounded metadata pass, without loading every retained scene JSON into memory.
      const rows: { key: IDBValidKey; bytes: number; used: number }[] = [];
      let bytes = record.bytes;
      const cursor = store.index("metadata").openKeyCursor();
      cursor.onsuccess = () => {
        const item = cursor.result;
        if (item) {
          const [projectId, revision, used, size] = item.key as [
            string,
            number,
            number,
            number,
          ];
          if (
            item.primaryKey === record.key ||
            (projectId === record.projectId && revision !== record.revision)
          )
            store.delete(item.primaryKey);
          else {
            rows.push({ key: item.primaryKey, bytes: size, used });
            bytes += size;
          }
          item.continue();
          return;
        }
        rows.sort((a, b) => a.used - b.used);
        while (
          rows.length &&
          (rows.length >= SCENE_CACHE_ENTRIES || bytes > SCENE_CACHE_BYTES)
        ) {
          const old = rows.shift()!;
          store.delete(old.key);
          bytes -= old.bytes;
        }
        store.put(record);
      };
    });
  },
};

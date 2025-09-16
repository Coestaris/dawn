import init, { run } from "./pkg/dawn_wasm.js";

function init_bd(db) {
    const store = db.createObjectStore("assets", {keyPath: "id"});
    store.createIndex("name", "name", {unique: true});
    store.createIndex("hash", "hash", {unique: false});
    store.createIndex("size", "size", {unique: false});
    store.createIndex("content", "content", {unique: false});
}

async function fetch_resources_from_server() {
    const response = await fetch("api/enumerate")
    const json = await response.json()
    return json.resources;
}

async function download_resource(resource) {
    const response = await fetch(`api/get?name=${resource.name}`)
    const blob = await response.blob()
    const arrayBuffer = await blob.arrayBuffer();
    return new Uint8Array(arrayBuffer)
}

async function fetch_resources_from_db(db) {
    const transaction = db.transaction("assets", "readonly");
    const store = transaction.objectStore("assets");
    return new Promise((resolve, reject) => {
        const request = store.getAll();
        request.onsuccess = function (event) {
            resolve(event.target.result);
        };
        request.onerror = function (event) {
            reject(event);
        };
    });
}

async function put_resource_in_db(db, resource) {
    console.log("Putting resource in DB:", resource);
    const transaction = db.transaction("assets", "readwrite");
    const store = transaction.objectStore("assets");
    return new Promise((resolve, reject) => {
        const request = store.put(resource);
        request.onsuccess = function (event) {
            resolve(event.target.result);
        };
        request.onerror = function (event) {
            reject(event);
        };
    });
}


async function update_resources_cache(db) {
    const server_resources = await fetch_resources_from_server();
    console.log("Fetched resources from Server:", server_resources);
    const db_resources = await fetch_resources_from_db(db);
    console.log("Fetched resources from DB:", db_resources);

    // If the server has some new resources or hashes doesn't match, update the DB
    let to_download = [];
    server_resources.forEach(server_resource => {
        let db_resource = db_resources.find(db_resource => db_resource.name === server_resource.name);
        if (!db_resource ||
            db_resource.hash !== server_resource.hash ||
            db_resource.size !== server_resource.size ||
            db_resource.content === null) {
            to_download.push(server_resource);
        }
    })

    if (to_download.length !== 0) {
        console.log("Resources to download:", to_download);
        let new_contents = {}
        for (const resource of to_download) {
            let content = await download_resource(resource);
            console.log("Downloaded resource:", content);
            new_contents[resource.name] = content;
        }

        console.log(new_contents);

        // Put the new resources in the DB
        server_resources.forEach(server_resource => {
            let db_resource = db_resources.find(db_resource => db_resource.name === server_resource.name);
            let object = {
                id: server_resource.name,
                name: server_resource.name,
                hash: server_resource.hash,
                size: server_resource.size,
                content: new_contents[server_resource.name] || (db_resource ? db_resource.content : null)
            }
            put_resource_in_db(db, object);
        })
    }
}

if (!window.indexedDB) {
    console.error("Your browser doesn't support a stable version of IndexedDB")
} else {
    const request = indexedDB.open("assets", 1);
    request.onupgradeneeded = function (event) {
        init_bd(event.target.result);
    };
    request.onsuccess = async function (event) {
        const db = event.target.result;

        await update_resources_cache(db);
        await init(new URL('./pkg/dawn_wasm_bg.wasm', import.meta.url));

        const resources = await fetch_resources_from_db(db);

        run(resources);
    };
    request.onerror = function (event) {
        console.error("Error opening IndexedDB", event);
    };
}


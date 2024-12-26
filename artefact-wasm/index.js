import init, { compute } from "./pkg/artefact_wasm.js";

void init();
document.querySelector("#submitBtn").addEventListener("click", async () => {
    const file = document.getElementById("file").files[0];
    if (!file) {
        console.error("No file selected");
        return;
    }

    console.log("file selected");

    const reader = new FileReader();
    reader.onload = (e) => {
        if (e.target === null || e.target.result === null) {
            console.error("No file loaded");
            return;
        }
        const result = compute(new Uint8Array(e.target.result));

        const blob = new Blob([result], { type: "image/png" });
        const url = URL.createObjectURL(blob);

        const img = document.createElement("img");
        img.src = url;

        document.body.appendChild(img);
    }
    reader.readAsArrayBuffer(file);
});
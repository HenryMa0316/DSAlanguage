import { EditorView, basicSetup } from "codemirror";
import { python } from "@codemirror/lang-python";
import init, { run } from "./pkg/z_orion.js";

const outputEl = document.getElementById("output");
const errorEl = document.getElementById("error");
const statsEl = document.getElementById("stats");
let wasmReady = false;

init().then(() => {
    wasmReady = true;
    console.log("WASM ready");
}).catch(err => {
    console.error("WASM load failed:", err);
    outputEl.textContent = "Failed to load WASM: " + err;
});

const editor = new EditorView({
    doc: `def bubble_sort(OrionList nums) -> OrionList:
    OrionInt n = len(nums)
    for (i in range(n)):
        if (nums[i] > nums[i + 1]):
            OrionInt tmp = nums[i]
            nums[i] = nums[i + 1]
            nums[i + 1] = tmp
    return nums

OrionList arr = [3, 1, 4, 1, 5]
print(bubble_sort(arr))
`,
    extensions: [basicSetup, python()],
    parent: document.getElementById("editor"),
});

document.getElementById("run").addEventListener("click", () => {
    if (!wasmReady) {
        outputEl.textContent = "WASM is still loading...";
        return;
    }

    const source = editor.state.doc.toString();
    const inputText = document.getElementById("stdin").value;

    try {
        const res = JSON.parse(run(source, inputText));
        outputEl.textContent = res.output.join("\n");

        if (res.error) {
            errorEl.style.display = "block";
            errorEl.textContent = `Line ${res.error.line}: ${res.error.message}`;
        } else {
            errorEl.style.display = "none";
        }

        if (res.stats) {
            statsEl.style.display = "block";
            statsEl.textContent =
                `Comparisons: ${res.stats.comparisons}\n` +
                `Assignments: ${res.stats.assignments}\n` +
                `Function calls: ${res.stats.function_calls}\n` +
                `Peak element count: ${res.stats.peak_element_count}`;
        } else {
            statsEl.style.display = "none";
        }
    } catch (e) {
        outputEl.textContent = "Error: " + e;
    }
});
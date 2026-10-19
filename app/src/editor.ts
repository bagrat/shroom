import { mount } from "svelte";
import { reportErrors } from "./lib/ipc";

reportErrors("editor");
import "./lib/theme.css";
import Editor from "./Editor.svelte";

export default mount(Editor, { target: document.getElementById("app")! });

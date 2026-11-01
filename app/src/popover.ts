import { mount } from "svelte";
import { reportErrors } from "./lib/ipc";

reportErrors("popover");
import "./lib/theme.css";
import "./lib/parts.css";
import Popover from "./Popover.svelte";

export default mount(Popover, { target: document.getElementById("app")! });

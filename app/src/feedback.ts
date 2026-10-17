import { mount } from "svelte";
import { reportErrors } from "./lib/ipc";

reportErrors("feedback");
import "./lib/theme.css";
import Feedback from "./Feedback.svelte";

export default mount(Feedback, { target: document.getElementById("app")! });

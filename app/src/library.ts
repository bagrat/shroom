import { mount } from "svelte";
import { reportErrors } from "./lib/ipc";

reportErrors("library");
import "./lib/theme.css";
import Library from "./Library.svelte";
import "./onboarding/onboarding.css";
import "./library/sheet.css";
import "./library/library.css";

export default mount(Library, { target: document.getElementById("app")! });

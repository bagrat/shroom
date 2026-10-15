import { mount } from "svelte";
import { reportErrors } from "./lib/ipc";

reportErrors("onboarding");
import "./lib/theme.css";
import Onboarding from "./Onboarding.svelte";
import "./onboarding/onboarding.css";
import "./library/sheet.css";

export default mount(Onboarding, { target: document.getElementById("app")! });

import { mount } from "svelte";
import { reportErrors } from "./lib/ipc";

reportErrors("settings");
import "./lib/theme.css";
import Settings from "./Settings.svelte";
import "./onboarding/onboarding.css";
import "./library/sheet.css";
import "./settings/settings.css";
import "./members/members.css";

export default mount(Settings, { target: document.getElementById("app")! });

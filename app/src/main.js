import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import IconSheet from "./lib/icons/IconSheet.svelte";
import Mouse3DSheet from "./devices/vt2/components/Mouse3DSheet.svelte";

document.documentElement.classList.add("dark");
const q = location.search;
const Root = q.includes("icons") ? IconSheet : q.includes("3d") ? Mouse3DSheet : App;
export default mount(Root, { target: document.getElementById("app") });
